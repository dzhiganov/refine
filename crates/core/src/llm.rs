//! One client for every provider: OpenRouter and OpenAI-compatible servers (Ollama, LM Studio,
//! OpenAI) all speak `/chat/completions` and `/models`.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::settings::{ProviderConfig, ProviderKind, Thinking};

/// How strictly the answer's shape is enforced by the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputMode {
    /// `response_format: json_schema` with `strict: true` (OpenAI structured outputs).
    JsonSchema,
    /// `response_format: json_object`; the schema goes in the prompt.
    JsonObject,
    /// No `response_format`; the schema goes in the prompt and JSON is extracted from the text.
    Plain,
}

impl OutputMode {
    pub fn weaker(self) -> Option<OutputMode> {
        match self {
            OutputMode::JsonSchema => Some(OutputMode::JsonObject),
            OutputMode::JsonObject => Some(OutputMode::Plain),
            OutputMode::Plain => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_length: Option<u32>,
    /// USD per million input tokens (OpenRouter only).
    pub prompt_price: Option<f64>,
    /// USD per million output tokens (OpenRouter only).
    pub completion_price: Option<f64>,
    /// Whether the model is known to support strict JSON schema output (`None` = unknown).
    pub structured_outputs: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: &'static str,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Message { role: "system", content: content.into() }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Message { role: "user", content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Message { role: "assistant", content: content.into() }
    }
}

pub struct JsonRequest<'a> {
    pub messages: &'a [Message],
    pub schema_name: &'a str,
    pub schema: &'a Value,
    pub mode: OutputMode,
    pub temperature: f32,
    /// Ask the model to skip reasoning (when the provider setting allows it).
    pub no_thinking: bool,
}

#[derive(Clone)]
pub struct LlmClient {
    http: reqwest::Client,
    provider: ProviderConfig,
    api_key: Option<String>,
}

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        // Local models on a laptop can take a while for a long text.
        .timeout(Duration::from_secs(180))
        .user_agent(concat!("Refine/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("HTTP client")
}

impl LlmClient {
    pub fn new(http: reqwest::Client, provider: ProviderConfig, api_key: Option<String>) -> Result<Self> {
        if provider.needs_key && api_key.as_deref().is_none_or(|k| k.trim().is_empty()) {
            return Err(Error::MissingKey(provider.label.clone()));
        }
        Ok(LlmClient { http, provider, api_key })
    }

    pub fn provider(&self) -> &ProviderConfig {
        &self.provider
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.provider.base_url.trim_end_matches('/'), path)
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut req = self.http.request(method, self.url(path));
        if let Some(key) = self.api_key.as_deref().filter(|k| !k.is_empty()) {
            req = req.bearer_auth(key);
        }
        if self.provider.kind == ProviderKind::OpenRouter {
            // App attribution: https://openrouter.ai/docs/api-reference/overview#headers
            req = req.header("HTTP-Referer", "https://github.com/dzhiganov/refine").header("X-Title", "Refine");
        }
        req
    }

    async fn send(&self, req: reqwest::RequestBuilder) -> Result<Value> {
        let resp = req.send().await.map_err(|e| {
            tracing::warn!(error = %e, "request failed");
            if e.is_timeout() {
                Error::Network(format!("{} (timed out)", self.provider.label))
            } else {
                Error::Network(self.provider.label.clone())
            }
        })?;
        let status = resp.status();
        let text = resp.text().await.map_err(|_| Error::Network(self.provider.label.clone()))?;
        if !status.is_success() {
            return Err(Error::Provider {
                provider: self.provider.label.clone(),
                status: status.as_u16(),
                message: error_message(&text),
            });
        }
        let body: Value = serde_json::from_str(&text)
            .map_err(|_| Error::InvalidOutput(format!("{} returned something that isn't JSON", self.provider.label)))?;
        // OpenRouter reports some upstream failures with HTTP 200 and an `error` object.
        if let Some(err) = body.get("error") {
            return Err(Error::Provider {
                provider: self.provider.label.clone(),
                status: err.get("code").and_then(Value::as_u64).unwrap_or(500) as u16,
                message: err.get("message").and_then(Value::as_str).unwrap_or("unknown error").to_string(),
            });
        }
        Ok(body)
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let body = self.send(self.request(reqwest::Method::GET, "models")).await?;
        let data = body.get("data").and_then(Value::as_array).cloned().unwrap_or_default();
        let per_million = |v: Option<&Value>| {
            v.and_then(|p| p.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| p.as_f64()))
                .filter(|p| *p >= 0.0)
                .map(|p| (p * 1_000_000.0 * 1000.0).round() / 1000.0)
        };
        let mut models: Vec<ModelInfo> = data
            .iter()
            .filter_map(|m| {
                let id = m.get("id")?.as_str()?.to_string();
                let params = m.get("supported_parameters").and_then(Value::as_array);
                Some(ModelInfo {
                    name: m.get("name").and_then(Value::as_str).unwrap_or(&id).to_string(),
                    context_length: m.get("context_length").and_then(Value::as_u64).map(|n| n as u32),
                    prompt_price: per_million(m.pointer("/pricing/prompt")),
                    completion_price: per_million(m.pointer("/pricing/completion")),
                    structured_outputs: params.map(|p| {
                        p.iter().any(|v| v.as_str() == Some("structured_outputs") || v.as_str() == Some("response_format"))
                    }),
                    id,
                })
            })
            .collect();
        models.sort_by_key(|m| m.name.to_lowercase());
        Ok(models)
    }

    /// Sends a chat request and returns the assistant's raw text.
    pub async fn complete_json(&self, req: &JsonRequest<'_>) -> Result<String> {
        if self.provider.model.trim().is_empty() {
            return Err(Error::NoModel(self.provider.label.clone()));
        }
        let messages: Vec<Value> = req.messages.iter().map(|m| json!({ "role": m.role, "content": m.content })).collect();
        let mut body = json!({
            "model": self.provider.model,
            "messages": messages,
            "temperature": req.temperature,
            "stream": false,
        });
        let effort = match self.provider.thinking {
            Thinking::Off => Some("none"),
            Thinking::Low => Some("low"),
            Thinking::ModelDefault => None,
        };
        if let Some(effort) = effort.filter(|_| req.no_thinking) {
            match self.provider.kind {
                // https://openrouter.ai/docs/use-cases/reasoning-tokens
                ProviderKind::OpenRouter if effort == "none" => body["reasoning"] = json!({ "enabled": false }),
                ProviderKind::OpenRouter => body["reasoning"] = json!({ "effort": effort }),
                // OpenAI-style; LM Studio and Ollama honour it for thinking models.
                ProviderKind::OpenAiCompatible => body["reasoning_effort"] = json!(effort),
            }
        }
        match req.mode {
            OutputMode::JsonSchema => {
                body["response_format"] = json!({
                    "type": "json_schema",
                    "json_schema": { "name": req.schema_name, "strict": true, "schema": req.schema },
                });
                if self.provider.kind == ProviderKind::OpenRouter {
                    // Only route to upstream providers that honour response_format.
                    body["provider"] = json!({ "require_parameters": true });
                }
            }
            OutputMode::JsonObject => body["response_format"] = json!({ "type": "json_object" }),
            OutputMode::Plain => {}
        }
        let resp = self.send(self.request(reqwest::Method::POST, "chat/completions").json(&body)).await?;
        let content = resp.pointer("/choices/0/message/content").and_then(Value::as_str).unwrap_or_default();
        if content.trim().is_empty() {
            let reason = resp.pointer("/choices/0/finish_reason").and_then(Value::as_str).unwrap_or("none");
            return Err(Error::InvalidOutput(format!("empty answer (finish reason: {reason})")));
        }
        Ok(content.to_string())
    }
}

/// True when a provider error means "this model/server doesn't support that output mode".
pub fn is_unsupported_format(err: &Error) -> bool {
    match err {
        Error::Provider { status, message, .. } => {
            let m = message.to_lowercase();
            matches!(status, 400 | 404 | 422 | 501)
                && ["response_format", "json_schema", "structured output", "json mode", "no endpoints found"]
                    .iter()
                    .any(|k| m.contains(k))
        }
        _ => false,
    }
}

/// True when a provider rejected the "don't think" parameter (some models/servers don't know it).
pub fn is_unsupported_reasoning(err: &Error) -> bool {
    match err {
        Error::Provider { status, message, .. } => matches!(status, 400 | 422) && message.to_lowercase().contains("reasoning"),
        _ => false,
    }
}

/// Pulls a readable message out of an error body (OpenAI, OpenRouter and Ollama shapes).
fn error_message(body: &str) -> String {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let msg = parsed.as_ref().and_then(|v| {
        v.pointer("/error/message")
            .or_else(|| v.get("error"))
            .or_else(|| v.get("message"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let msg = msg.unwrap_or_else(|| body.trim().to_string());
    let msg: String = msg.chars().take(400).collect();
    if msg.is_empty() { "no details".into() } else { msg }
}

/// Extracts the first JSON object from model text (handles ```json fences and chatter around it).
pub fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for (i, ch) in text[start..].char_indices() {
        if in_str {
            match ch {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_str = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..start + i + 1]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_json_from_fenced_text() {
        let t = "Sure!\n```json\n{\"a\": \"}{\", \"b\": {\"c\": 1}}\n```\nDone.";
        assert_eq!(extract_json(t), Some("{\"a\": \"}{\", \"b\": {\"c\": 1}}"));
        assert_eq!(extract_json("no json"), None);
    }

    #[test]
    fn detects_unsupported_format_errors() {
        let e = Error::Provider {
            provider: "x".into(),
            status: 400,
            message: "Invalid parameter: response_format of type 'json_schema' is not supported".into(),
        };
        assert!(is_unsupported_format(&e));
        let e = Error::Provider { provider: "x".into(), status: 401, message: "bad key".into() };
        assert!(!is_unsupported_format(&e));
    }

    #[test]
    fn reads_error_messages() {
        assert_eq!(error_message(r#"{"error":{"message":"No auth"}}"#), "No auth");
        assert_eq!(error_message(r#"{"error":"model not found"}"#), "model not found");
        assert_eq!(error_message("plain"), "plain");
    }
}

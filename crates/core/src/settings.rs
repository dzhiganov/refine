use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::Cefr;
use crate::tone::DEFAULT_TONE_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    /// openrouter.ai: one key, hundreds of models.
    OpenRouter,
    /// Anything speaking the OpenAI chat-completions API: Ollama, LM Studio, OpenAI, vLLM...
    OpenAiCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub kind: ProviderKind,
    pub label: String,
    /// Base URL ending before `/chat/completions`, e.g. `http://localhost:11434/v1`.
    pub base_url: String,
    /// Model id; empty until the user picks one.
    pub model: String,
    /// Local servers usually don't need a key.
    pub needs_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LanguageProfile {
    /// ISO 639-1 code: "en", "de".
    pub code: String,
    /// English name, used in prompts: "English", "German".
    pub name: String,
    pub level: Cefr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    System,
    Dark,
    Light,
}

/// When the app may spend tokens on the written "what to work on" summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct InsightsPolicy {
    /// Generate automatically while idle (otherwise only on request).
    pub auto: bool,
    /// At least this many new texts since the last summary.
    pub min_new_entries: u32,
    /// And at least this many hours since the last summary.
    pub min_hours_between: u32,
}

// `default`: settings saved by an older version (missing newer fields) still load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub active_provider_id: String,
    pub providers: Vec<ProviderConfig>,
    pub languages: Vec<LanguageProfile>,
    /// Language the explanations are written in (a code from `languages` or any ISO code).
    pub explanation_language: String,
    /// "auto" or a language code.
    pub language_mode: String,
    pub tone_id: String,
    pub theme: Theme,
    pub insights: InsightsPolicy,
    /// Minutes without activity before background stats work starts.
    pub idle_minutes: u32,
}

impl Default for InsightsPolicy {
    fn default() -> Self {
        InsightsPolicy { auto: true, min_new_entries: 5, min_hours_between: 24 }
    }
}

impl Default for Settings {
    fn default() -> Self {
        let provider = |id: &str, kind, label: &str, base_url: &str, needs_key| ProviderConfig {
            id: id.into(),
            kind,
            label: label.into(),
            base_url: base_url.into(),
            model: String::new(),
            needs_key,
        };
        Settings {
            active_provider_id: "openrouter".into(),
            providers: vec![
                provider("openrouter", ProviderKind::OpenRouter, "OpenRouter", "https://openrouter.ai/api/v1", true),
                provider("ollama", ProviderKind::OpenAiCompatible, "Ollama", "http://localhost:11434/v1", false),
                provider("lmstudio", ProviderKind::OpenAiCompatible, "LM Studio", "http://localhost:1234/v1", false),
                provider("openai", ProviderKind::OpenAiCompatible, "OpenAI", "https://api.openai.com/v1", true),
            ],
            languages: vec![
                LanguageProfile { code: "en".into(), name: "English".into(), level: Cefr::B2 },
                LanguageProfile { code: "de".into(), name: "German".into(), level: Cefr::B1 },
            ],
            explanation_language: "en".into(),
            language_mode: "auto".into(),
            tone_id: DEFAULT_TONE_ID.into(),
            theme: Theme::System,
            insights: InsightsPolicy::default(),
            idle_minutes: 3,
        }
    }
}

impl Settings {
    pub fn provider(&self, id: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.id == id)
    }

    pub fn active_provider(&self) -> Option<&ProviderConfig> {
        self.provider(&self.active_provider_id)
    }

    pub fn language(&self, code: &str) -> Option<&LanguageProfile> {
        self.languages.iter().find(|l| l.code == code)
    }

    pub fn language_name(&self, code: &str) -> String {
        self.language(code).map(|l| l.name.clone()).unwrap_or_else(|| code.to_string())
    }

    /// Rejects settings the rest of the app can't work with.
    pub fn validate(&self) -> Result<(), String> {
        if self.languages.is_empty() {
            return Err("Keep at least one language.".into());
        }
        if self.active_provider().is_none() {
            return Err("The active provider doesn't exist.".into());
        }
        let mut ids: Vec<&str> = self.providers.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err("Provider ids must be unique.".into());
        }
        for p in &self.providers {
            if !(p.base_url.starts_with("http://") || p.base_url.starts_with("https://")) {
                return Err(format!("{}: the address must start with http:// or https://", p.label));
            }
        }
        if self.language_mode != "auto" && self.language(&self.language_mode).is_none() {
            return Err("The selected language isn't in your language list.".into());
        }
        Ok(())
    }
}

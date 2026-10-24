//! The written summary: an LLM turns the finished stats JSON into a few sentences and one tip.
//! It only phrases what the numbers say; every number it quotes is checked against the JSON (§4.4).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::llm::{JsonRequest, LlmClient, Message, OutputMode, extract_json, is_unsupported_format, is_unsupported_reasoning};
use crate::model::{ModelRef, SCHEMA_VERSION};
use crate::settings::Settings;
use crate::stats::{LanguageStats, Stats};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LanguageInsight {
    pub language: String,
    /// At most three sentences about progress and what to focus on.
    pub summary: String,
    /// The watch-list pattern the tip is about, if any.
    pub focus_pattern_key: Option<String>,
    /// One concrete rule (rule patterns) or item to remember (item patterns), with an example.
    pub tip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Insights {
    pub schema_version: u32,
    pub generated_at: DateTime<Utc>,
    /// `Stats.entryCount` the summary was written from.
    #[ts(type = "number")]
    pub entry_count: u64,
    pub model: ModelRef,
    pub languages: Vec<LanguageInsight>,
    /// Every number in the text was found in the stats it was written from.
    pub numbers_verified: bool,
}

/// The slice of the stats the model sees: enough to write from, small enough to be cheap.
fn compact(l: &LanguageStats) -> Value {
    let watch: Vec<Value> = l
        .patterns
        .iter()
        .filter(|p| p.on_watch_list)
        .map(|p| {
            json!({
                "key": p.key,
                "label": p.label,
                "category": p.category,
                "kind": p.kind,
                "state": p.state,
                "daysWithError30d": p.days_with_error_30d,
                "recentAccuracy": p.recent_accuracy,
                "opportunitiesRecent": p.opportunities_recent,
                "example": p.examples.first().map(|e| json!({ "sentence": e.sentence, "original": e.original, "correction": e.correction })),
            })
        })
        .collect();
    let areas: Vec<Value> = l
        .areas
        .iter()
        .take(3)
        .map(|a| {
            json!({
                "area": a.area,
                "share": a.share,
                "trend": a.rate.trend,
                "topCategories": a.categories.iter().take(3).map(|c| json!({ "category": c.category, "share": c.share, "trend": c.rate.trend })).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "language": l.language,
        "languageName": l.language_name,
        "texts": l.totals.texts,
        "words": l.totals.words,
        "activeWeeksOfLast12": l.totals.active_weeks_12,
        "errorsPer100Words": { "recent": l.error_rate.recent, "prior": l.error_rate.prior, "trend": l.error_rate.trend },
        "meaningErrorsPer100Words": { "recent": l.global_error_rate.recent, "prior": l.global_error_rate.prior, "trend": l.global_error_rate.trend },
        "areas": areas,
        "watchList": watch,
        "masteredRecently": l.mastered_recently.iter().map(|m| &m.label).collect::<Vec<_>>(),
        "suggestions": {
            "offered": l.suggestions.offered,
            "reusedLater": l.suggestions.reused,
            "recentlyReused": l.suggestions.recent_uptakes.iter().map(|u| &u.phrase).collect::<Vec<_>>(),
        },
    })
}

fn schema(languages: &[&str], keys: &[&str]) -> Value {
    let mut keys: Vec<&str> = keys.to_vec();
    keys.push("");
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["languages"],
        "properties": {
            "languages": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["language", "summary", "focusPatternKey", "tip"],
                    "properties": {
                        "language": { "type": "string", "enum": languages },
                        "summary": { "type": "string" },
                        "focusPatternKey": { "type": "string", "enum": keys },
                        "tip": { "type": "string" }
                    }
                }
            }
        }
    })
}

fn system_prompt(explain_in: &str) -> String {
    format!(
        r#"You write a short progress note for an adult learning languages by writing real texts (chat messages, emails, comments). You get their stats as JSON, computed by the app. Write in {explain_in}.

For each language in the data, return:
- summary: at most 3 short sentences. Lead with what improved or held steady, then the one thing to focus on next. Talk about the writing, not the person ("article errors dropped", not "you are bad at articles"). Mention a trend only if its `trend` field says improving or worsening; "stable" means no real change. If there is too little data, say so kindly in one sentence.
- focusPatternKey: the key of the watch-list pattern to focus on (prefer relapsed, then recurring), or "" if the watch list is empty.
- tip: for that pattern, one concrete, memorable line. For kind "rule", state the rule in everyday words with a tiny example. For kind "item", give the exact word or phrase to remember. You may reuse the learner's own example sentence. Empty string if there is no focus pattern.

Rules:
- Use only numbers that appear in the JSON. Write shares (0–1) as percentages (0.25 → 25%); write everything else exactly as it appears. Never compute new numbers or totals.
- Never mention total mistake counts, scores, grades, CEFR level estimates, comparisons with other people, or streaks.
- Warm and matter-of-fact. No exclamation marks, no emoji.

Return only the JSON object."#
    )
}

/// Numbers in `text` that don't appear in the stats JSON (as written, or as a percentage of a share).
fn unverified_numbers(text: &str, stats_json: &str) -> Vec<String> {
    let allowed: Vec<f64> = numbers(stats_json).iter().filter_map(|n| n.parse().ok()).collect();
    numbers(text)
        .into_iter()
        .filter(|n| {
            let Ok(v) = n.parse::<f64>() else { return false };
            // Small counting words ("2 sentences", "one tip") are harmless.
            if v.fract() == 0.0 && v <= 3.0 {
                return false;
            }
            !allowed.iter().any(|a| (a - v).abs() < 1e-9 || (a * 100.0 - v).abs() < 0.5)
        })
        .collect()
}

fn numbers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        let decimal_point = (*c == '.' || *c == ',') && !cur.is_empty() && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit());
        if c.is_ascii_digit() || decimal_point {
            cur.push(if *c == ',' { '.' } else { *c });
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInsights {
    languages: Vec<RawLanguage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLanguage {
    language: String,
    summary: String,
    #[serde(default)]
    focus_pattern_key: String,
    #[serde(default)]
    tip: String,
}

pub async fn generate_insights(client: &LlmClient, settings: &Settings, stats: &Stats, now: DateTime<Utc>) -> Result<Insights> {
    let langs: Vec<&LanguageStats> = stats.languages.iter().filter(|l| l.totals.texts > 0).collect();
    if langs.is_empty() {
        return Err(Error::Invalid("Check a few texts first; the summary is written from your history.".into()));
    }
    let data = Value::Array(langs.iter().map(|l| compact(l)).collect());
    let data_json = serde_json::to_string(&data)?;
    let codes: Vec<&str> = langs.iter().map(|l| l.language.as_str()).collect();
    let keys: Vec<&str> = langs.iter().flat_map(|l| l.watch_list.iter().map(String::as_str)).collect();
    let schema = schema(&codes, &keys);
    let system = system_prompt(&settings.language_name(&settings.explanation_language));
    let mut messages = vec![Message::system(system.clone()), Message::user(format!("<stats>\n{data_json}\n</stats>"))];

    let mut mode = OutputMode::JsonSchema;
    let mut no_thinking = true;
    let mut attempt = 0;
    loop {
        if mode != OutputMode::JsonSchema {
            messages[0] = Message::system(format!("{system}{}", crate::prompt::schema_instructions(&schema)));
        }
        let req = JsonRequest { messages: &messages, schema_name: "refine_insights", schema: &schema, mode, temperature: 0.4, no_thinking };
        let text = match client.complete_json(&req).await {
            Ok(t) => t,
            Err(e) if no_thinking && is_unsupported_reasoning(&e) => {
                no_thinking = false;
                continue;
            }
            Err(e) if is_unsupported_format(&e) && mode.weaker().is_some() => {
                mode = mode.weaker().unwrap();
                continue;
            }
            Err(e) => return Err(e),
        };
        attempt += 1;
        let raw: RawInsights = extract_json(&text)
            .and_then(|j| serde_json::from_str(j).ok())
            .ok_or_else(|| Error::InvalidOutput("the summary isn't valid JSON".into()))?;
        let languages: Vec<LanguageInsight> = raw
            .languages
            .into_iter()
            .filter(|l| codes.contains(&l.language.as_str()))
            .map(|l| LanguageInsight {
                focus_pattern_key: Some(l.focus_pattern_key).filter(|k| keys.contains(&k.as_str())),
                language: l.language,
                summary: l.summary.trim().to_string(),
                tip: l.tip.trim().to_string(),
            })
            .collect();
        let bad: Vec<String> = languages
            .iter()
            .flat_map(|l| unverified_numbers(&format!("{} {}", l.summary, l.tip), &data_json))
            .collect();
        if bad.is_empty() || attempt >= 2 {
            let provider = client.provider();
            return Ok(Insights {
                schema_version: SCHEMA_VERSION,
                generated_at: now,
                entry_count: stats.entry_count,
                model: ModelRef { provider_id: provider.id.clone(), provider_kind: provider.kind, model: provider.model.clone() },
                languages,
                numbers_verified: bad.is_empty(),
            });
        }
        messages.truncate(2);
        messages.push(Message::assistant(text));
        messages.push(Message::user(format!(
            "These numbers are not in the stats: {}. Use only numbers that appear in the JSON, or none. Return the full JSON again.",
            bad.join(", ")
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_invented_numbers() {
        let json = r#"{"recent":2.1,"prior":3.4,"share":0.26,"texts":42}"#;
        assert!(unverified_numbers("Down from 3.4 to 2.1 per 100 words over 42 texts.", json).contains(&"100".to_string()));
        assert!(unverified_numbers("Down from 3,4 to 2.1; 26% of errors.", json).is_empty());
        assert_eq!(unverified_numbers("You improved by 38%.", json), vec!["38"]);
        assert!(unverified_numbers("Focus on 1 thing.", json).is_empty());
    }
}

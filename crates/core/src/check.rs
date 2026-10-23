//! A text check end to end: prompt → model → parse → validate (one repair round) → `Entry`.

use std::time::Instant;

use chrono::Utc;
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::llm::{JsonRequest, LlmClient, Message, OutputMode, extract_json, is_unsupported_format};
use crate::model::{Cefr, Entry, Mistake, ModelRef, PatternCheck, SCHEMA_VERSION, Suggestion, SuggestionKind, ToneRef, pattern_key};
use crate::prompt::{CHECK_SCHEMA_NAME, CheckContext, check_schema, check_system_prompt, check_user_prompt, schema_instructions};
use crate::taxonomy::ErrorCategory;
use crate::text::{find_from, find_word_from, to_span, word_count, word_diff};

pub const MAX_INPUT_CHARS: usize = 20_000;

/// The model's answer as parsed, before validation. Lenient on purpose: strictness is enforced
/// by the schema (when the server supports it) and by `validate`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOutput {
    #[serde(default)]
    language: String,
    #[serde(default)]
    mistakes: Vec<RawMistake>,
    fixed_text: String,
    #[serde(default)]
    suggestions: Vec<RawSuggestion>,
    #[serde(default)]
    watch: Vec<RawWatch>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawWatch {
    pattern: String,
    #[serde(default)]
    correct: i64,
    #[serde(default)]
    errors: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMistake {
    original: String,
    correction: String,
    #[serde(default)]
    explanation: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    pattern: String,
    #[serde(default)]
    impedes_meaning: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct RawSuggestion {
    #[serde(default)]
    kind: String,
    original: String,
    replacement: String,
    #[serde(default)]
    explanation: String,
    #[serde(default)]
    level: String,
}

pub struct CheckOutcome {
    pub entry: Entry,
    /// The output mode that worked, to remember for this model.
    pub mode: OutputMode,
}

pub async fn run_check(client: &LlmClient, ctx: &CheckContext<'_>, text: &str, mode: OutputMode) -> Result<CheckOutcome> {
    let input = text.trim_end();
    if input.trim().is_empty() {
        return Err(Error::Invalid("Paste some text first.".into()));
    }
    if input.chars().count() > MAX_INPUT_CHARS {
        return Err(Error::Invalid(format!("That's a long one. Check up to {MAX_INPUT_CHARS} characters at a time.")));
    }
    let started = Instant::now();
    let schema = check_schema(ctx);
    let base_system = check_system_prompt(ctx);
    let mut messages = vec![Message::system(String::new()), Message::user(check_user_prompt(input))];
    let mut mode = mode;

    let mut attempt = 0;
    let mut warnings = Vec::new();
    loop {
        messages[0] = Message::system(match mode {
            OutputMode::JsonSchema => base_system.clone(),
            _ => format!("{base_system}{}", schema_instructions(&schema)),
        });
        let req = JsonRequest { messages: &messages, schema_name: CHECK_SCHEMA_NAME, schema: &schema, mode, temperature: 0.2 };
        let raw_text = match client.complete_json(&req).await {
            Ok(t) => t,
            Err(e) if is_unsupported_format(&e) && mode.weaker().is_some() => {
                tracing::info!(?mode, error = %e, "output mode unsupported, falling back");
                mode = mode.weaker().unwrap();
                continue;
            }
            Err(e) => return Err(e),
        };
        attempt += 1;

        let problems = match parse(&raw_text) {
            Ok(raw) => {
                let validated = validate(raw, input, ctx);
                // A language the learner hasn't set up: don't file it under another one.
                if let Some(lang) = &validated.unknown_language
                    && (lang == "other" || attempt >= 2)
                {
                    let known: Vec<String> = ctx.settings.languages.iter().map(|l| l.name.clone()).collect();
                    let which = if lang == "other" || lang.is_empty() { String::new() } else { format!(" (it looks like \"{lang}\")") };
                    return Err(Error::Invalid(format!(
                        "This text isn't in {}{which}. Add its language in Settings → Languages to check it.",
                        known.join(" or ")
                    )));
                }
                if validated.problems.is_empty() || attempt >= 2 {
                    let mut validated = validated;
                    warnings.extend(std::mem::take(&mut validated.problems));
                    let entry = build_entry(client, ctx, input, validated, warnings, started);
                    return Ok(CheckOutcome { entry, mode });
                }
                validated.problems
            }
            Err(e) if attempt >= 2 => return Err(e),
            Err(e) => vec![e.to_string()],
        };

        // One repair round: show the model its answer and what was wrong with it.
        tracing::info!(?problems, "asking the model to repair its answer");
        messages.truncate(2);
        messages.push(Message::assistant(raw_text));
        messages.push(Message::user(format!(
            "Your answer has problems:\n- {}\nReturn the complete corrected JSON object.",
            problems.join("\n- ")
        )));
    }
}

fn parse(text: &str) -> Result<RawOutput> {
    let json = extract_json(text).ok_or_else(|| Error::InvalidOutput("the answer contains no JSON object".into()))?;
    serde_json::from_str(json).map_err(|e| Error::InvalidOutput(format!("the JSON doesn't have the expected fields ({e})")))
}

struct Validated {
    language: String,
    /// Set when the model reported a language that isn't configured ("other" or a code).
    unknown_language: Option<String>,
    fixed_text: String,
    mistakes: Vec<Mistake>,
    suggestions: Vec<Suggestion>,
    watch: Vec<PatternCheck>,
    problems: Vec<String>,
}

fn validate(raw: RawOutput, input: &str, ctx: &CheckContext) -> Validated {
    let mut problems = Vec::new();
    let codes = ctx.language_codes();
    let mut unknown_language = None;
    let language = if codes.contains(&raw.language.as_str()) {
        raw.language.clone()
    } else {
        problems.push(format!("`language` must be one of: {} (or \"other\")", codes.join(", ")));
        unknown_language = Some(raw.language.clone());
        codes[0].to_string()
    };

    let mut fixed_text = raw.fixed_text.trim_end().to_string();
    if fixed_text.trim().is_empty() {
        problems.push("`fixedText` is empty".into());
        fixed_text = input.to_string();
    }

    let allowed = ctx.categories();
    let mut mistakes = Vec::new();
    // Locate in text order even if the model listed them out of order.
    let mut raw_mistakes: Vec<RawMistake> =
        raw.mistakes.into_iter().filter(|m| !m.original.trim().is_empty() && m.original != m.correction).collect();
    raw_mistakes.sort_by_key(|m| find_word_from(input, &m.original, 0).map_or(usize::MAX, |r| r.0));
    let mut cursor = 0;
    for m in raw_mistakes {
        let Some(range) = find_word_from(input, &m.original, cursor) else {
            problems.push(format!("mistake `original` \"{}\" is not copied verbatim from the text", m.original));
            continue;
        };
        if range.0 < cursor {
            problems.push(format!("mistake \"{}\" overlaps another mistake; list each fragment once", m.original));
            continue;
        }
        cursor = range.1;
        let category = match ErrorCategory::parse(&m.category) {
            Some(c) if allowed.contains(&c) => c,
            _ => {
                problems.push(format!("\"{}\" is not an allowed category", m.category));
                ErrorCategory::fallback(&m.category)
            }
        };
        let mut pattern = m.pattern.trim().to_lowercase();
        if pattern.is_empty() {
            // Keep the stats usable: fall back to the item itself.
            pattern = crate::text::normalize(&m.correction);
        }
        mistakes.push(Mistake {
            id: format!("m{}", mistakes.len() + 1),
            pattern_key: pattern_key(&language, category, &pattern),
            pattern,
            original: m.original,
            correction: m.correction,
            category,
            impedes_meaning: m.impedes_meaning,
            explanation: m.explanation.trim().to_string(),
            span: Some(to_span(input, range)),
        });
    }
    if mistakes.is_empty() && fixed_text != input {
        problems.push("`fixedText` differs from the input but no mistakes are listed; list every change, or return the input unchanged".into());
        // A rewrite with no listed mistakes is never shown as a correction.
        fixed_text = input.to_string();
    }
    for m in &mistakes {
        if !m.correction.is_empty() && !fixed_text.contains(&m.correction) {
            problems.push(format!("the correction \"{}\" doesn't appear in `fixedText`", m.correction));
        }
    }

    // Suggestions at most one CEFR level above the learner (§3.7).
    let ceiling = ctx.settings.language(&language).map(|l| l.level.next()).unwrap_or(Cefr::C2);
    let mut suggestions = Vec::new();
    for s in raw.suggestions.into_iter() {
        if suggestions.len() == 3 {
            break;
        }
        if s.original.trim().is_empty() || s.original == s.replacement {
            continue;
        }
        let level = Cefr::parse(&s.level).unwrap_or(Cefr::B2);
        if level > ceiling {
            continue;
        }
        let span = find_from(&fixed_text, &s.original, 0).map(|r| to_span(&fixed_text, r));
        if span.is_none() {
            problems.push(format!("suggestion `original` \"{}\" is not copied verbatim from `fixedText`", s.original));
            continue;
        }
        suggestions.push(Suggestion {
            id: format!("s{}", suggestions.len() + 1),
            kind: match s.kind.as_str() {
                "collocation" => SuggestionKind::Collocation,
                "synonym" => SuggestionKind::Synonym,
                _ => SuggestionKind::Phrase,
            },
            original: s.original,
            replacement: s.replacement,
            explanation: s.explanation.trim().to_string(),
            level,
            span,
        });
    }

    // Opportunity counts: only for watched patterns of this text's language, clamped to sane
    // values, and never fewer errors than the mistakes actually listed for that pattern.
    let mut watch = Vec::new();
    for w in ctx.watched.iter().filter(|w| w.language == language) {
        let listed = mistakes.iter().filter(|m| m.pattern_key == w.key).count() as u32;
        let reported = raw.watch.iter().find(|r| r.pattern == w.label);
        let (correct, errors) = reported.map_or((0, 0), |r| (r.correct.clamp(0, 50) as u32, r.errors.clamp(0, 50) as u32));
        watch.push(PatternCheck { key: w.key.clone(), label: w.label.clone(), correct, errors: errors.max(listed) });
    }

    Validated { language, unknown_language, fixed_text, mistakes, suggestions, watch, problems }
}

fn build_entry(
    client: &LlmClient,
    ctx: &CheckContext,
    input: &str,
    v: Validated,
    warnings: Vec<String>,
    started: Instant,
) -> Entry {
    let settings = ctx.settings;
    let level = settings.language(&v.language).map(|l| l.level).unwrap_or(Cefr::B1);
    let fixed_text = v.fixed_text;
    let provider = client.provider();
    Entry {
        schema_version: SCHEMA_VERSION,
        id: uuid::Uuid::now_v7().to_string(),
        created_at: Utc::now(),
        input: input.to_string(),
        language: v.language,
        level,
        tone: ToneRef { id: ctx.tone.id.clone(), name: ctx.tone.name.clone() },
        model: ModelRef { provider_id: provider.id.clone(), provider_kind: provider.kind, model: provider.model.clone() },
        diff: word_diff(input, &fixed_text),
        fixed_text,
        mistakes: v.mistakes,
        suggestions: v.suggestions,
        watch: v.watch,
        word_count: word_count(input),
        duration_ms: started.elapsed().as_millis() as u64,
        warnings,
    }
}

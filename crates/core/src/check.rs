//! A text check end to end: prompt → model → parse → validate (one repair round) → `Entry`.

use std::time::Instant;

use chrono::Utc;
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::llm::{JsonRequest, LlmClient, Message, OutputMode, extract_json, is_unsupported_format, is_unsupported_reasoning};
use crate::model::{Cefr, Entry, Mistake, ModelRef, PatternCheck, SCHEMA_VERSION, Suggestion, SuggestionKind, ToneRef, pattern_key};
use crate::prompt::{CHECK_SCHEMA_NAME, CheckContext, check_schema, check_system_prompt, check_user_prompt, schema_instructions};
use crate::taxonomy::ErrorCategory;
use crate::text::{expand_ellipsis, find_from, find_word_from, to_span, word_count, word_diff};

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
    /// The model's re-read of its own fixed text: "ok" or what's still wrong.
    #[serde(default)]
    review: String,
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

/// What a model turned out to support, remembered per model so fallbacks aren't repeated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelCaps {
    pub mode: OutputMode,
    /// The model accepts the "don't think" parameter.
    pub no_thinking: bool,
}

impl Default for ModelCaps {
    fn default() -> Self {
        ModelCaps { mode: OutputMode::JsonSchema, no_thinking: true }
    }
}

pub struct CheckOutcome {
    pub entry: Entry,
    /// What worked, to remember for this model.
    pub caps: ModelCaps,
}

pub async fn run_check(client: &LlmClient, ctx: &CheckContext<'_>, text: &str, caps: ModelCaps) -> Result<CheckOutcome> {
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
    let ModelCaps { mut mode, mut no_thinking } = caps;

    let mut attempt = 0;
    let mut warnings = Vec::new();
    loop {
        messages[0] = Message::system(match mode {
            OutputMode::JsonSchema => base_system.clone(),
            _ => format!("{base_system}{}", schema_instructions(&schema)),
        });
        let req = JsonRequest { messages: &messages, schema_name: CHECK_SCHEMA_NAME, schema: &schema, mode, temperature: 0.2, no_thinking };
        let raw_text = match client.complete_json(&req).await {
            Ok(t) => t,
            Err(e) if no_thinking && is_unsupported_reasoning(&e) => {
                tracing::info!(error = %e, "model rejects the reasoning parameter; retrying without it");
                no_thinking = false;
                continue;
            }
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
                    return Ok(CheckOutcome { entry, caps: ModelCaps { mode, no_thinking } });
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

    let model_fixed = raw.fixed_text.trim_end().to_string();
    if model_fixed.trim().is_empty() {
        problems.push("`fixedText` is empty".into());
    }

    let allowed = ctx.categories();
    let mut mistakes = Vec::new();
    // Locate in text order even if the model listed them out of order.
    let mut raw_mistakes: Vec<RawMistake> =
        raw.mistakes.into_iter().filter(|m| !m.original.trim().is_empty() && m.original != m.correction).collect();
    // "machen ... eine Entscheidung": expand to the full fragment if it's not verbatim.
    for m in raw_mistakes.iter_mut() {
        if find_word_from(input, &m.original, 0).is_none()
            && let Some((_, original, correction)) = expand_ellipsis(input, &m.original, &m.correction, 0)
        {
            m.original = original;
            m.correction = correction;
        }
    }
    raw_mistakes.sort_by_key(|m| find_word_from(input, &m.original, 0).map_or(usize::MAX, |r| r.0));
    let mut cursor = 0;
    let mut ranges = Vec::new();
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
        ranges.push(range);
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
    let review = raw.review.trim();
    let review_ok = review.is_empty() || review.trim_matches(|c: char| !c.is_alphanumeric()).eq_ignore_ascii_case("ok");
    if !review_ok {
        problems.push(format!("your review of fixedText found a problem: \"{review}\". Fix the `mistakes` (and corrections) so the fixed text is fully correct"));
    }

    // The fixed text is rebuilt from the input and the listed corrections, so nothing the model
    // didn't list (and explain) can change. Its own version only tells us what it forgot to list.
    let fixed_text = apply_corrections(input, &ranges, &mistakes);
    if !model_fixed.trim().is_empty() && crate::text::normalize(&model_fixed) != crate::text::normalize(&fixed_text) {
        let extra: Vec<String> = word_diff(&fixed_text, &model_fixed)
            .into_iter()
            .filter(|p| p.op != crate::model::DiffOp::Equal && !p.text.trim().is_empty())
            .take(6)
            .map(|p| format!("{} \"{}\"", if p.op == crate::model::DiffOp::Insert { "added" } else { "removed" }, p.text.trim()))
            .collect();
        problems.push(format!(
            "`fixedText` has changes that aren't listed in `mistakes` ({}). List each real error as a mistake, and leave everything else exactly as written",
            extra.join(", ")
        ));
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

/// The input with each mistake's byte range replaced by its correction (ranges are sorted and
/// don't overlap; `validate` guarantees both).
fn apply_corrections(input: &str, ranges: &[(usize, usize)], mistakes: &[Mistake]) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last = 0;
    for ((start, end), m) in ranges.iter().zip(mistakes) {
        out.push_str(&input[last..*start]);
        out.push_str(&m.correction);
        last = *end;
    }
    out.push_str(&input[last..]);
    out
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

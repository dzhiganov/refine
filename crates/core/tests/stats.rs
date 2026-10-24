//! Lifecycle, rates and uptake on synthetic histories. Thresholds: docs/research/stats-system.md §4.

use chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};
use refine_core::model::{Entry, Mistake, ModelRef, PatternCheck, SCHEMA_VERSION, Suggestion, SuggestionKind, ToneRef, pattern_key};
use refine_core::settings::{ProviderKind, Settings};
use refine_core::stats::{PatternPrefs, PatternState, ReviewRecord, Stats, Trend, compute_stats};
use refine_core::taxonomy::ErrorCategory;

const MIT: &str = "dative after mit";

fn key(label: &str) -> String {
    pattern_key("de", ErrorCategory::Case, label)
}

fn day(n: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap() + Duration::days(n)
}

struct E {
    at: DateTime<Utc>,
    words: u32,
    errors: Vec<(&'static str, ErrorCategory)>,
    watch: Option<(u32, u32)>,
    input: String,
    suggestions: Vec<&'static str>,
}

fn e(at: i64) -> E {
    E { at: day(at), words: 100, errors: vec![], watch: None, input: String::new(), suggestions: vec![] }
}

impl E {
    fn err(mut self, label: &'static str) -> Self {
        self.errors.push((label, ErrorCategory::Case));
        self
    }
    fn errs(mut self, n: usize, label: &'static str, cat: ErrorCategory) -> Self {
        for _ in 0..n {
            self.errors.push((label, cat));
        }
        self
    }
    fn watch(mut self, correct: u32, errors: u32) -> Self {
        self.watch = Some((correct, errors));
        self
    }
    fn words(mut self, n: u32) -> Self {
        self.words = n;
        self
    }
    fn input(mut self, s: &str) -> Self {
        self.input = s.into();
        self
    }
    fn suggest(mut self, s: &'static str) -> Self {
        self.suggestions.push(s);
        self
    }
    fn build(self, i: usize) -> Entry {
        let mistakes = self
            .errors
            .iter()
            .enumerate()
            .map(|(j, (label, cat))| Mistake {
                id: format!("m{j}"),
                original: "mit der Mann".into(),
                correction: "mit dem Mann".into(),
                category: *cat,
                impedes_meaning: false,
                explanation: String::new(),
                pattern: label.to_string(),
                pattern_key: pattern_key("de", *cat, label),
                span: None,
            })
            .collect();
        Entry {
            schema_version: SCHEMA_VERSION,
            id: format!("e{i:03}"),
            created_at: self.at,
            input: self.input,
            language: "de".into(),
            tone: ToneRef { id: "slack".into(), name: "Slack".into() },
            model: ModelRef { provider_id: "t".into(), provider_kind: ProviderKind::OpenRouter, model: "m".into() },
            fixed_text: String::new(),
            mistakes,
            suggestions: self
                .suggestions
                .iter()
                .enumerate()
                .map(|(j, s)| Suggestion {
                    id: format!("s{j}"),
                    kind: SuggestionKind::Collocation,
                    original: "x".into(),
                    replacement: s.to_string(),
                    explanation: String::new(),
                    span: None,
                })
                .collect(),
            watch: self
                .watch
                .map(|(correct, errors)| vec![PatternCheck { key: key(MIT), label: MIT.into(), correct, errors }])
                .unwrap_or_default(),
            diff: vec![],
            word_count: self.words,
            duration_ms: 0,
            warnings: vec![],
        }
    }
}

fn stats(entries: Vec<E>, now: i64) -> Stats {
    stats_with(entries, now, &PatternPrefs::default(), &[])
}

fn stats_with(entries: Vec<E>, now: i64, prefs: &PatternPrefs, reviews: &[ReviewRecord]) -> Stats {
    stats_tz(entries, day(now), prefs, reviews, FixedOffset::east_opt(0).unwrap())
}

fn stats_tz(entries: Vec<E>, now: DateTime<Utc>, prefs: &PatternPrefs, reviews: &[ReviewRecord], tz: FixedOffset) -> Stats {
    let entries: Vec<Entry> = entries.into_iter().enumerate().map(|(i, e)| e.build(i)).collect();
    compute_stats(&entries, &Settings::default(), prefs, reviews, tz, now)
}

fn state(s: &Stats, label: &str) -> Option<PatternState> {
    s.languages[0].patterns.iter().find(|p| p.label == label).map(|p| p.state)
}

#[test]
fn one_long_text_does_not_make_a_pattern_recurring() {
    // Five errors in one text, and two texts on the same day: still a probable slip (§3.3).
    let s = stats(vec![e(0).err(MIT).err(MIT).err(MIT), e(0).err(MIT)], 1);
    assert_eq!(state(&s, MIT), Some(PatternState::New));
    assert!(s.languages[0].watch_list.is_empty());
}

#[test]
fn two_texts_on_two_days_make_it_recurring_and_watched() {
    let s = stats(vec![e(0).err(MIT), e(3).err(MIT)], 4);
    assert_eq!(state(&s, MIT), Some(PatternState::Recurring));
    assert_eq!(s.languages[0].watch_list, vec![key(MIT)]);
    let p = &s.languages[0].patterns[0];
    assert_eq!((p.texts_with_error, p.days_with_error), (2, 2));
    // Reminder ladder: last error on day 3, step 0 → due a day later.
    let r = p.reminder.as_ref().unwrap();
    assert_eq!((r.step, r.next_due, r.due), (0, day(4), true));
}

#[test]
fn errors_more_than_30_days_apart_are_not_recurring() {
    let s = stats(vec![e(0).err(MIT), e(40).err(MIT)], 41);
    assert_eq!(state(&s, MIT), Some(PatternState::New));
}

#[test]
fn one_offs_are_archived_after_30_days() {
    let s = stats(vec![e(0).err(MIT)], 31);
    assert_eq!(state(&s, MIT), None, "archived patterns are hidden");
}

#[test]
fn full_lifecycle_improving_mastered_relapsed() {
    let mut h = vec![e(0).err(MIT), e(2).err(MIT)];
    // Watched now: three opportunity texts, 8 of 9 correct (≥ 75%, ≥ 4 uses) → improving.
    h.push(e(5).watch(3, 0));
    h.push(e(6).watch(2, 1).err(MIT));
    h.push(e(8).watch(3, 0));
    let s = stats(h, 9);
    assert_eq!(state(&s, MIT), Some(PatternState::Improving));
    let p = &s.languages[0].patterns[0];
    assert_eq!(p.recent_accuracy, Some(0.889));
    assert_eq!(p.opportunities_recent, 9);

    // Three clean texts in a row at ≥ 90% with ≥ 6 uses, and ≥ 14 days since the last error.
    let mut h = vec![e(0).err(MIT), e(2).err(MIT), e(5).watch(3, 0), e(6).watch(2, 1).err(MIT), e(8).watch(3, 0)];
    h.extend([e(12).watch(2, 0), e(16).watch(3, 0), e(21).watch(2, 0)]);
    // Day 21 is 15 days after the last error (day 6).
    let s = stats(h, 21);
    assert_eq!(state(&s, MIT), Some(PatternState::Mastered));
    assert_eq!(s.languages[0].mastered_recently.len(), 1);
    assert!(s.languages[0].watch_list.is_empty());

    // A new error after mastery → relapsed, back on the watch list.
    let mut h = vec![e(0).err(MIT), e(2).err(MIT), e(5).watch(3, 0), e(6).watch(2, 1).err(MIT), e(8).watch(3, 0)];
    h.extend([e(12).watch(2, 0), e(16).watch(3, 0), e(21).watch(2, 0), e(30).err(MIT)]);
    let s = stats(h, 31);
    assert_eq!(state(&s, MIT), Some(PatternState::Relapsed));
    assert_eq!(s.languages[0].watch_list, vec![key(MIT)]);
}

#[test]
fn silence_never_means_mastered() {
    // Recurring, then nothing for 61 days → dormant, not mastered (§4.3).
    let s = stats(vec![e(0).err(MIT), e(2).err(MIT)], 64);
    assert_eq!(state(&s, MIT), Some(PatternState::Dormant));
}

#[test]
fn not_mastered_before_the_14_day_gap() {
    let h = vec![e(0).err(MIT), e(2).err(MIT), e(3).watch(3, 0), e(4).watch(3, 0), e(5).watch(3, 0)];
    let s = stats(h, 10);
    assert_eq!(state(&s, MIT), Some(PatternState::Improving));
    let h = vec![e(0).err(MIT), e(2).err(MIT), e(3).watch(3, 0), e(4).watch(3, 0), e(5).watch(3, 0)];
    let s = stats(h, 17);
    assert_eq!(state(&s, MIT), Some(PatternState::Mastered));
}

#[test]
fn watch_list_is_capped_and_respects_pins_and_dismissals() {
    let labels = ["p1", "p2", "p3", "p4"];
    let mut h = Vec::new();
    for (i, l) in labels.iter().enumerate() {
        // p1 recurs on the most days, then p2...
        for d in 0..(5 - i as i64) {
            h.push(e(d * 2).err(l));
        }
    }
    let s = stats(h, 12);
    let watch: Vec<String> = s.languages[0].watch_list.clone();
    assert_eq!(watch, vec![key("p1"), key("p2"), key("p3")]);

    let mut h = Vec::new();
    for (i, l) in labels.iter().enumerate() {
        for d in 0..(5 - i as i64) {
            h.push(e(d * 2).err(l));
        }
    }
    let prefs = PatternPrefs { pinned: vec![key("p4")], dismissed: vec![key("p1")] };
    let s = stats_with(h, 12, &prefs, &[]);
    assert_eq!(s.languages[0].watch_list, vec![key("p4"), key("p2"), key("p3")]);
}

#[test]
fn reviews_climb_the_reminder_ladder() {
    let reviews = vec![
        ReviewRecord { pattern_key: key(MIT), at: day(4), passed: true },
        ReviewRecord { pattern_key: key(MIT), at: day(7), passed: true },
    ];
    let s = stats_with(vec![e(0).err(MIT), e(3).err(MIT)], 8, &PatternPrefs::default(), &reviews);
    let r = s.languages[0].patterns[0].reminder.clone().unwrap();
    // Two passes → step 2 → 7 days after the last review.
    assert_eq!((r.step, r.next_due, r.due), (2, day(14), false));
}

#[test]
fn error_rate_trend_needs_a_real_difference() {
    // 30 texts × 100 words. Older half: 6 errors per text; recent half: 1 per text.
    let mut h = Vec::new();
    for i in 0..15 {
        h.push(e(i).errs(6, "x", ErrorCategory::Article));
    }
    for i in 15..30 {
        h.push(e(i).errs(1, "y", ErrorCategory::Article));
    }
    let s = stats(h, 30);
    let r = &s.languages[0].error_rate;
    assert_eq!((r.recent_words, r.prior_words), (1500, 1500));
    assert_eq!((r.recent, r.prior), (Some(1.0), Some(6.0)));
    assert_eq!(r.trend, Trend::Improving);
    assert_eq!(s.languages[0].areas[0].categories[0].category, ErrorCategory::Article);

    // A small wobble is "stable".
    let mut h = Vec::new();
    for i in 0..30 {
        h.push(e(i).errs(if i % 2 == 0 { 2 } else { 3 }, "x", ErrorCategory::Article));
    }
    assert_eq!(stats(h, 30).languages[0].error_rate.trend, Trend::Stable);
}

#[test]
fn rates_need_enough_words_and_skip_short_texts() {
    // 4 texts × 100 words: 400 < 500 → no rate. Short texts don't count at all.
    let h = vec![e(0).err(MIT), e(1), e(2), e(3), e(4).words(10).err(MIT)];
    let s = stats(h, 5);
    let l = &s.languages[0];
    assert_eq!(l.error_rate.recent, None);
    assert_eq!(l.error_rate.trend, Trend::InsufficientData);
    assert_eq!(l.totals.rated_words, 400);
    assert_eq!(l.totals.words, 410);
}

#[test]
fn suggestion_uptake_counts_later_own_use() {
    let h = vec![
        e(0).input("Ich mache eine Entscheidung.").suggest("eine Entscheidung treffen"),
        e(1).input("Wir müssen bald eine Entscheidung treffen."),
        e(2).input("Hallo").suggest("Bescheid geben"),
    ];
    let s = stats(h, 3);
    let u = &s.languages[0].suggestions;
    assert_eq!((u.offered, u.reused), (2, 1));
    assert_eq!(u.uptake_rate, Some(0.5));
    assert_eq!(u.recent_uptakes[0].phrase, "eine Entscheidung treffen");
}

#[test]
fn stats_serialize_with_camel_case_keys() {
    let s = stats(vec![e(0).err(MIT), e(3).err(MIT)], 4);
    let v = serde_json::to_value(&s).unwrap();
    assert!(v["languages"][0]["watchList"].is_array());
    assert_eq!(v["languages"][0]["patterns"][0]["state"], "recurring");
    assert_eq!(v["thresholds"]["ladderDays"], serde_json::json!([1, 3, 7, 14, 30, 60]));
}

fn mastered_history() -> Vec<E> {
    vec![e(0).err(MIT), e(2).err(MIT), e(5).watch(3, 0), e(6).watch(2, 1).err(MIT), e(8).watch(3, 0), e(12).watch(2, 0), e(16).watch(3, 0), e(21).watch(2, 0)]
}

#[test]
fn a_relapse_is_not_remastered_on_old_evidence() {
    // Review finding: after a relapse, the old clean texts used to re-promote the pattern.
    let mut h = mastered_history();
    h.extend([e(30).err(MIT), e(31).watch(0, 0)]);
    assert_eq!(state(&stats(h, 31), MIT), Some(PatternState::Relapsed));
    let mut h = mastered_history();
    h.extend([e(30).err(MIT), e(31).watch(0, 0)]);
    assert_eq!(state(&stats(h, 45), MIT), Some(PatternState::Relapsed), "silence after a relapse isn't mastery");

    // Fresh evidence after the relapse does count.
    let mut h = mastered_history();
    h.extend([e(30).err(MIT), e(32).watch(3, 0), e(34).watch(2, 0), e(36).watch(2, 0)]);
    assert_eq!(state(&stats(h, 37), MIT), Some(PatternState::Improving));
}

#[test]
fn long_silence_makes_a_pattern_dormant_before_the_next_error() {
    // Recurring on days 0 and 2, dormant from day 62; an error on day 100 starts it over as new.
    let s = stats(vec![e(0).err(MIT), e(2).err(MIT), e(100).err(MIT)], 101);
    let p = &s.languages[0].patterns[0];
    assert_eq!(p.state, PatternState::New);
    assert_eq!(p.since, day(100));
}

#[test]
fn days_are_the_learners_local_days() {
    // 23:50 and 00:10 UTC are the same evening in New York (UTC-5): one session, not two days.
    let late = Utc.with_ymd_and_hms(2026, 1, 1, 23, 50, 0).unwrap();
    let h = vec![E { at: late, ..e(0).err(MIT) }, E { at: late + Duration::minutes(20), ..e(0).err(MIT) }];
    let ny = FixedOffset::west_opt(5 * 3600).unwrap();
    let s = stats_tz(h, late + Duration::hours(1), &PatternPrefs::default(), &[], ny);
    assert_eq!(state(&s, MIT), Some(PatternState::New));
    let h = vec![E { at: late, ..e(0).err(MIT) }, E { at: late + Duration::minutes(20), ..e(0).err(MIT) }];
    let s = stats_tz(h, late + Duration::hours(1), &PatternPrefs::default(), &[], FixedOffset::east_opt(0).unwrap());
    assert_eq!(state(&s, MIT), Some(PatternState::Recurring), "in UTC these are two days");
}

#[test]
fn rechecking_the_same_text_is_not_uptake() {
    let h = vec![
        e(0).input("Ich mache eine Entscheidung.").suggest("eine Entscheidung treffen"),
        // Same day: the learner edited the text and checked again.
        E { at: day(0) + Duration::minutes(5), ..e(0).input("Ich muss eine Entscheidung treffen.") },
    ];
    assert_eq!(stats(h, 1).languages[0].suggestions.reused, 0);
}

#[test]
fn uptake_needs_whole_words() {
    let h = vec![e(0).input("So.").suggest("thus far"), e(1).input("Enthusiastic about thus farming.")];
    assert_eq!(stats(h, 2).languages[0].suggestions.reused, 0);
}

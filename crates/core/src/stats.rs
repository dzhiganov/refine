//! Deterministic learning stats (no LLM): rates, trends, error-pattern lifecycle, watch list,
//! reminder ladder and suggestion uptake. Design and evidence: `docs/research/stats-system.md` §4.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::{Entry, SCHEMA_VERSION, SuggestionKind};
use crate::settings::Settings;
use crate::taxonomy::{ErrorArea, ErrorCategory, ErrorKind};
use crate::text::normalize;

/// Every threshold the stats use, published in the JSON so each number can be explained.
/// The ones marked "design choice" are reasoned, not empirically calibrated (§6).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Thresholds {
    /// Words per comparison window (design choice, §4.1).
    pub window_words: u32,
    /// A rate is shown only over at least this many words (§4.1, §4.5).
    pub min_window_words: u32,
    /// Shorter texts feed patterns but not rates.
    pub min_text_words: u32,
    /// p-value below which a change counts as a trend (exact Poisson rate-ratio test).
    pub significance: f64,
    /// Recurring: errors in this many distinct texts...
    pub recurring_texts: u32,
    /// ...on this many distinct days...
    pub recurring_days: u32,
    /// ...within this many days (Corder's errors vs slips, §3.3).
    pub recurring_window_days: u32,
    /// Improving: pooled accuracy over the last `mastered_texts` opportunity texts (design choice).
    pub improving_accuracy: f64,
    pub improving_min_opportunities: u32,
    /// Mastered: accuracy in each of `mastered_texts` consecutive opportunity texts (Brown 1973).
    pub mastered_accuracy: f64,
    pub mastered_texts: u32,
    pub mastered_min_opportunities: u32,
    /// And this many days without an error for the pattern (stand-in for a delayed post-test).
    pub mastered_gap_days: u32,
    pub dormant_days: u32,
    /// `new` patterns that don't repeat within this many days are archived.
    pub archive_days: u32,
    pub watch_list_size: u32,
    /// Expanding reminder ladder in days (§3.5).
    pub ladder_days: Vec<u32>,
    /// Suggestion uptake looks back this many days.
    pub uptake_days: u32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            window_words: 1500,
            min_window_words: 500,
            min_text_words: 20,
            significance: 0.05,
            recurring_texts: 2,
            recurring_days: 2,
            recurring_window_days: 30,
            improving_accuracy: 0.75,
            improving_min_opportunities: 4,
            mastered_accuracy: 0.9,
            mastered_texts: 3,
            mastered_min_opportunities: 6,
            mastered_gap_days: 14,
            dormant_days: 60,
            archive_days: 30,
            watch_list_size: 3,
            ladder_days: vec![1, 3, 7, 14, 30, 60],
            uptake_days: 90,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Trend {
    Improving,
    Stable,
    Worsening,
    InsufficientData,
}

/// Errors per 100 words in the most recent window vs the one before it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RateTrend {
    /// Errors per 100 words in the recent window; `None` under `minWindowWords`.
    pub recent: Option<f64>,
    pub prior: Option<f64>,
    pub recent_words: u32,
    pub prior_words: u32,
    pub recent_errors: u32,
    pub prior_errors: u32,
    pub rate_ratio: Option<f64>,
    pub p_value: Option<f64>,
    pub trend: Trend,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ToneRate {
    pub tone_id: String,
    pub tone_name: String,
    pub texts: u32,
    pub words: u32,
    pub error_rate: RateTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CategoryStats {
    pub category: ErrorCategory,
    pub kind: Option<ErrorKind>,
    /// Share of all errors in the recent window (0–1). "Where your errors are", not a judgement.
    pub share: f64,
    pub rate: RateTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AreaStats {
    pub area: ErrorArea,
    pub share: f64,
    pub rate: RateTrend,
    /// Only categories with errors in the two windows, most frequent first.
    pub categories: Vec<CategoryStats>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum PatternState {
    New,
    Recurring,
    Improving,
    Mastered,
    Relapsed,
    Dormant,
    /// A one-off that never came back; kept for history, not shown.
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PatternExample {
    pub entry_id: String,
    pub at: DateTime<Utc>,
    pub original: String,
    pub correction: String,
    /// The sentence from the learner's text that contained the mistake (for "fix your own sentence").
    pub sentence: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    /// Index into `ladderDays`.
    pub step: u32,
    pub next_due: DateTime<Utc>,
    pub due: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PatternStats {
    pub key: String,
    pub label: String,
    pub category: ErrorCategory,
    pub kind: Option<ErrorKind>,
    pub state: PatternState,
    /// When the current state began.
    pub since: DateTime<Utc>,
    pub first_seen: DateTime<Utc>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub texts_with_error: u32,
    pub days_with_error: u32,
    pub days_with_error_30d: u32,
    /// Share of this pattern's errors that impeded meaning (0–1).
    pub impedes_meaning_share: f64,
    /// Pooled accuracy over the last opportunity texts; `None` until the pattern was watched.
    pub recent_accuracy: Option<f64>,
    pub opportunities_recent: u32,
    /// Newest first, at most 3.
    pub examples: Vec<PatternExample>,
    pub reminder: Option<Reminder>,
    pub on_watch_list: bool,
    pub pinned: bool,
    /// The learner said "not now"; kept off the watch list until they undo it.
    pub dismissed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MasteredItem {
    pub key: String,
    pub label: String,
    pub mastered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Uptake {
    pub phrase: String,
    pub suggested_at: DateTime<Utc>,
    pub reused_at: DateTime<Utc>,
    pub entry_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct KindCount {
    pub kind: SuggestionKind,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionStats {
    /// Distinct suggested phrases in the uptake window.
    pub offered: u32,
    /// Of those, reused later in the learner's own (uncorrected) writing.
    pub reused: u32,
    pub uptake_rate: Option<f64>,
    pub by_kind: Vec<KindCount>,
    /// Newest first, at most 5.
    pub recent_uptakes: Vec<Uptake>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub texts: u32,
    pub words: u32,
    /// Words in texts long enough to count towards rates.
    pub rated_words: u32,
    /// Of the last 12 weeks, how many had any writing (a soft count, never a streak).
    pub active_weeks_12: u32,
    pub first_at: Option<DateTime<Utc>>,
    pub last_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LanguageStats {
    pub language: String,
    pub language_name: String,
    pub totals: Totals,
    pub error_rate: RateTrend,
    /// Errors that impeded meaning ("global" errors), per 100 words. Kept apart, never weighted in.
    pub global_error_rate: RateTrend,
    pub by_tone: Vec<ToneRate>,
    pub areas: Vec<AreaStats>,
    /// Every visible pattern (archived ones left out), watch list first.
    pub patterns: Vec<PatternStats>,
    /// Pattern keys, at most `watchListSize` (+ pinned).
    pub watch_list: Vec<String>,
    /// Mastered in the last 30 days.
    pub mastered_recently: Vec<MasteredItem>,
    pub suggestions: SuggestionStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub schema_version: u32,
    pub generated_at: DateTime<Utc>,
    /// Number of entries the stats were computed from (all languages).
    #[ts(type = "number")]
    pub entry_count: u64,
    /// Store version the stats were computed at; any change to entries or settings bumps it.
    #[serde(default)]
    #[ts(type = "number")]
    pub data_version: u64,
    pub thresholds: Thresholds,
    pub languages: Vec<LanguageStats>,
}

/// The learner's own choices about patterns (autonomy, §3.8).
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct PatternPrefs {
    /// Always on the watch list.
    pub pinned: Vec<String>,
    /// Never on the watch list.
    pub dismissed: Vec<String>,
}

/// A "fix your own sentence" micro-review, self-graded.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRecord {
    pub pattern_key: String,
    pub at: DateTime<Utc>,
    pub passed: bool,
}

// ---- rates & trends --------------------------------------------------------------------

/// Natural log of the gamma function (Lanczos, g = 7), good to ~15 digits for x > 0.
fn ln_gamma(x: f64) -> f64 {
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = G[0];
    let t = x + 7.5;
    for (i, g) in G.iter().enumerate().skip(1) {
        a += g / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

fn binom_pmf(k: u32, n: u32, p: f64) -> f64 {
    if p <= 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    if p >= 1.0 {
        return if k == n { 1.0 } else { 0.0 };
    }
    let (k, n) = (f64::from(k), f64::from(n));
    (ln_gamma(n + 1.0) - ln_gamma(k + 1.0) - ln_gamma(n - k + 1.0) + k * p.ln() + (n - k) * (1.0 - p).ln()).exp()
}

/// Exact conditional test for two Poisson rates (Przyborowski & Wilenski): given
/// n = e1 + e2 errors, e1 ~ Binomial(n, w1 / (w1 + w2)) under "same rate". Two-sided p-value by
/// the doubled smaller tail, capped at 1.
pub fn poisson_rate_test(e1: u32, w1: u32, e2: u32, w2: u32) -> f64 {
    let n = e1 + e2;
    if n == 0 || w1 == 0 || w2 == 0 {
        return 1.0;
    }
    let p = f64::from(w1) / f64::from(w1 + w2);
    let lower: f64 = (0..=e1).map(|k| binom_pmf(k, n, p)).sum();
    let upper: f64 = (e1..=n).map(|k| binom_pmf(k, n, p)).sum();
    (2.0 * lower.min(upper)).min(1.0)
}

fn per100(errors: u32, words: u32) -> f64 {
    (f64::from(errors) * 100.0 / f64::from(words) * 100.0).round() / 100.0
}

/// One text as the rate code sees it.
struct RatedText<'a> {
    entry: &'a Entry,
    words: u32,
}

/// Splits rated texts (oldest first) into the recent and prior word windows.
fn windows<'a, 'b>(texts: &'b [RatedText<'a>], t: &Thresholds) -> (&'b [RatedText<'a>], &'b [RatedText<'a>]) {
    let total: u32 = texts.iter().map(|x| x.words).sum();
    // Two full windows when there's enough writing, otherwise split what there is in half.
    let target = t.window_words.min(total / 2).max(1);
    let take_back = |slice: &'b [RatedText<'a>]| -> usize {
        let mut words = 0;
        let mut n = 0;
        for x in slice.iter().rev() {
            if words >= target {
                break;
            }
            words += x.words;
            n += 1;
        }
        n
    };
    let n_recent = take_back(texts);
    let (rest, recent) = texts.split_at(texts.len() - n_recent);
    let n_prior = take_back(rest);
    let prior = &rest[rest.len() - n_prior..];
    (recent, prior)
}

fn rate_trend<'a>(texts: &[RatedText<'a>], t: &Thresholds, count: impl Fn(&'a Entry) -> u32) -> RateTrend {
    let (recent, prior) = windows(texts, t);
    let sum = |w: &[RatedText<'a>]| -> (u32, u32) {
        w.iter().fold((0, 0), |(e, n), x| (e + count(x.entry), n + x.words))
    };
    let (re, rw) = sum(recent);
    let (pe, pw) = sum(prior);
    let shown = |e, w| (w >= t.min_window_words).then(|| per100(e, w));
    let recent_rate = shown(re, rw);
    let prior_rate = shown(pe, pw);
    let (rate_ratio, p_value, trend) = match (recent_rate, prior_rate) {
        (Some(r), Some(p)) => {
            let ratio = if p > 0.0 { Some(((r / p) * 100.0).round() / 100.0) } else { None };
            let pv = poisson_rate_test(re, rw, pe, pw);
            let trend = if pv < t.significance {
                if per100(re, rw) < per100(pe, pw) { Trend::Improving } else { Trend::Worsening }
            } else {
                Trend::Stable
            };
            (ratio, Some((pv * 10_000.0).round() / 10_000.0), trend)
        }
        _ => (None, None, Trend::InsufficientData),
    };
    RateTrend {
        recent: recent_rate,
        prior: prior_rate,
        recent_words: rw,
        prior_words: pw,
        recent_errors: re,
        prior_errors: pe,
        rate_ratio,
        p_value,
        trend,
    }
}

// ---- patterns ----------------------------------------------------------------------------

/// The sentence around a byte range: from the previous sentence end to the next one.
fn sentence_around(text: &str, start: usize, end: usize) -> String {
    let is_end = |c: char| matches!(c, '.' | '!' | '?' | '\n');
    let from = text[..start].rfind(is_end).map(|i| i + 1).unwrap_or(0);
    let to = text[end..].find(is_end).map(|i| end + i + 1).unwrap_or(text.len());
    text[from..to].trim().to_string()
}

fn utf16_to_byte(text: &str, offset: u32) -> usize {
    let mut units = 0u32;
    for (i, ch) in text.char_indices() {
        if units >= offset {
            return i;
        }
        units += ch.len_utf16() as u32;
    }
    text.len()
}

#[derive(Clone, Copy)]
struct OppText {
    at: DateTime<Utc>,
    correct: u32,
    errors: u32,
}

impl OppText {
    fn n(&self) -> u32 {
        self.correct + self.errors
    }
    fn accuracy(&self) -> f64 {
        f64::from(self.correct) / f64::from(self.n().max(1))
    }
}

/// The learner's calendar day (stats count days and weeks in local time, not UTC).
fn local_day(at: DateTime<Utc>, tz: &FixedOffset) -> NaiveDate {
    at.with_timezone(tz).date_naive()
}

/// Walks one pattern's history and applies the lifecycle in §4.3.
struct Lifecycle<'t> {
    t: &'t Thresholds,
    tz: FixedOffset,
    state: Option<PatternState>,
    since: DateTime<Utc>,
    errors: Vec<(DateTime<Utc>, String)>, // (time, entry id)
    /// Opportunity texts since the current evidence window began (reset on relapse).
    opps: Vec<OppText>,
    /// Every opportunity text, for the reminder ladder.
    all_opps: Vec<OppText>,
    mastered_at: Option<DateTime<Utc>>,
}

impl<'t> Lifecycle<'t> {
    fn new(t: &'t Thresholds, tz: FixedOffset) -> Self {
        Lifecycle {
            t,
            tz,
            state: None,
            since: DateTime::<Utc>::MIN_UTC,
            errors: vec![],
            opps: vec![],
            all_opps: vec![],
            mastered_at: None,
        }
    }

    fn set(&mut self, state: PatternState, at: DateTime<Utc>) {
        if self.state != Some(state) {
            self.state = Some(state);
            self.since = at;
            match state {
                PatternState::Mastered => self.mastered_at = Some(at),
                // Mastery was earned on old evidence; a relapse needs fresh evidence (§4.3).
                PatternState::Relapsed => self.opps.clear(),
                _ => {}
            }
        }
    }

    fn is_recurring_at(&self, at: DateTime<Utc>) -> bool {
        let from = at - Duration::days(i64::from(self.t.recurring_window_days));
        let recent: Vec<_> = self.errors.iter().filter(|(when, _)| *when >= from && *when <= at).collect();
        let texts: BTreeSet<&String> = recent.iter().map(|(_, id)| id).collect();
        let days: BTreeSet<NaiveDate> = recent.iter().map(|(when, _)| local_day(*when, &self.tz)).collect();
        texts.len() as u32 >= self.t.recurring_texts && days.len() as u32 >= self.t.recurring_days
    }

    fn last_opps(&self) -> &[OppText] {
        let n = self.t.mastered_texts as usize;
        &self.opps[self.opps.len().saturating_sub(n)..]
    }

    fn pooled_accuracy(&self) -> Option<(f64, u32)> {
        let last = self.last_opps();
        if last.is_empty() {
            return None;
        }
        let (c, n) = last.iter().fold((0, 0), |(c, n), o| (c + o.correct, n + o.n()));
        Some((f64::from(c) / f64::from(n.max(1)), n))
    }

    fn meets_improving(&self) -> bool {
        self.opps.len() as u32 >= self.t.mastered_texts
            && self.pooled_accuracy().is_some_and(|(acc, n)| acc >= self.t.improving_accuracy && n >= self.t.improving_min_opportunities)
    }

    fn meets_mastered(&self, now: DateTime<Utc>) -> bool {
        let run: Vec<&OppText> =
            self.opps.iter().rev().take_while(|o| o.accuracy() >= self.t.mastered_accuracy).collect();
        let total: u32 = run.iter().map(|o| o.n()).sum();
        let quiet = self.errors.last().is_none_or(|(when, _)| now - *when >= Duration::days(i64::from(self.t.mastered_gap_days)));
        run.len() as u32 >= self.t.mastered_texts && total >= self.t.mastered_min_opportunities && quiet
    }

    fn on_text(&mut self, at: DateTime<Utc>, entry_id: &str, errors: u32, opp: Option<(u32, u32)>) {
        // Gaps first: a pattern may have gone dormant or been archived before this text.
        self.tick(at);
        if errors > 0 {
            self.errors.push((at, entry_id.to_string()));
        }
        let opp = match opp.filter(|(c, e)| c + e > 0) {
            Some((correct, errs)) => Some(OppText { at, correct, errors: errs }),
            // An error with no opportunity report still counts as a failed use, so a pattern
            // can't look accurate while its errors go uncounted.
            None if errors > 0 && self.state.is_some() => Some(OppText { at, correct: 0, errors }),
            None => None,
        };
        if let Some(o) = opp {
            self.all_opps.push(o);
            self.opps.push(o);
        }
        use PatternState::*;
        match self.state {
            None if errors > 0 => self.set(New, at),
            None => {}
            Some(Mastered) if errors > 0 => self.set(Relapsed, at),
            Some(New | Archived | Dormant) if self.is_recurring_at(at) => self.set(Recurring, at),
            Some(Archived | Dormant) if errors > 0 => self.set(New, at),
            Some(Recurring | Relapsed) if self.meets_improving() => self.set(Improving, at),
            Some(Improving) if self.meets_mastered(at) => self.set(Mastered, at),
            Some(Improving) if !self.meets_improving() => self.set(Recurring, at),
            _ => {}
        }
    }

    /// Time-based transitions at `now`: archive one-offs, mark long silence dormant, and
    /// complete mastery once the error-free gap has passed.
    fn tick(&mut self, now: DateTime<Utc>) {
        use PatternState::*;
        let last_event = self.errors.last().map(|e| e.0).into_iter().chain(self.all_opps.last().map(|o| o.at)).max();
        match self.state {
            Some(Improving) if self.meets_mastered(now) => self.set(Mastered, now),
            Some(New) if self.errors.last().is_some_and(|(at, _)| now - *at > Duration::days(i64::from(self.t.archive_days))) => {
                let at = self.errors.last().map(|e| e.0).unwrap_or(now) + Duration::days(i64::from(self.t.archive_days));
                self.set(Archived, at.min(now));
            }
            Some(Recurring | Improving | Relapsed)
                if last_event.is_some_and(|at| now - at > Duration::days(i64::from(self.t.dormant_days))) =>
            {
                let at = last_event.unwrap() + Duration::days(i64::from(self.t.dormant_days));
                self.set(Dormant, at);
            }
            _ => {}
        }
    }
}

/// Reminder ladder: a recurrence (or a failed review) resets to step 0; a clean opportunity
/// text (or a passed review) climbs one step. Due = anchor + ladder[step] days.
fn reminder(
    key: &str,
    errors: &[(DateTime<Utc>, String)],
    opps: &[OppText],
    reviews: &[ReviewRecord],
    t: &Thresholds,
    now: DateTime<Utc>,
) -> Option<Reminder> {
    let mut events: Vec<(DateTime<Utc>, bool)> = errors.iter().map(|(at, _)| (*at, false)).collect();
    events.extend(opps.iter().filter(|o| o.errors == 0).map(|o| (o.at, true)));
    events.extend(reviews.iter().filter(|r| r.pattern_key == key).map(|r| (r.at, r.passed)));
    events.sort_by_key(|e| e.0);
    let max_step = t.ladder_days.len().saturating_sub(1) as u32;
    let mut step = 0u32;
    let mut anchor = events.first()?.0;
    for (at, good) in events {
        step = if good { (step + 1).min(max_step) } else { 0 };
        anchor = at;
    }
    let days = t.ladder_days.get(step as usize).copied().unwrap_or(1);
    let next_due = anchor + Duration::days(i64::from(days));
    Some(Reminder { step, next_due, due: now >= next_due })
}

// ---- suggestions -------------------------------------------------------------------------

/// `phrase` occurs in `text` as whole words (both already normalized).
fn contains_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + phrase.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

fn suggestion_stats(entries: &[&Entry], t: &Thresholds, tz: &FixedOffset, now: DateTime<Utc>) -> SuggestionStats {
    let from = now - Duration::days(i64::from(t.uptake_days));
    let mut first_offer: BTreeMap<String, (DateTime<Utc>, String, usize)> = BTreeMap::new(); // phrase → (when, display, entry idx)
    let mut by_kind: BTreeMap<SuggestionKind, BTreeSet<String>> = BTreeMap::new();
    for (i, e) in entries.iter().enumerate().filter(|(_, e)| e.created_at >= from) {
        for s in &e.suggestions {
            let phrase = normalize(&s.replacement);
            // Very short replacements match by accident too often.
            if phrase.chars().count() < 4 || contains_phrase(&normalize(&e.input), &phrase) {
                continue;
            }
            by_kind.entry(s.kind).or_default().insert(phrase.clone());
            first_offer.entry(phrase).or_insert((e.created_at, s.replacement.clone(), i));
        }
    }
    let mut uptakes = Vec::new();
    for (phrase, (suggested_at, display, idx)) in &first_offer {
        // Reuse on a later day only: editing and re-checking the same text isn't uptake.
        let offered_on = local_day(*suggested_at, tz);
        if let Some(later) = entries[idx + 1..]
            .iter()
            .find(|e| local_day(e.created_at, tz) > offered_on && contains_phrase(&normalize(&e.input), phrase))
        {
            uptakes.push(Uptake {
                phrase: display.clone(),
                suggested_at: *suggested_at,
                reused_at: later.created_at,
                entry_id: later.id.clone(),
            });
        }
    }
    uptakes.sort_by_key(|u| std::cmp::Reverse(u.reused_at));
    let offered = first_offer.len() as u32;
    let reused = uptakes.len() as u32;
    SuggestionStats {
        offered,
        reused,
        uptake_rate: (offered > 0).then(|| ((f64::from(reused) / f64::from(offered)) * 1000.0).round() / 1000.0),
        by_kind: by_kind.into_iter().map(|(kind, set)| KindCount { kind, count: set.len() as u32 }).collect(),
        recent_uptakes: uptakes.into_iter().take(5).collect(),
    }
}

// ---- assembly ----------------------------------------------------------------------------

fn share(part: u32, total: u32) -> f64 {
    if total == 0 { 0.0 } else { ((f64::from(part) / f64::from(total)) * 1000.0).round() / 1000.0 }
}

/// Inputs shared by every language's stats.
struct Ctx<'a> {
    settings: &'a Settings,
    prefs: &'a PatternPrefs,
    reviews: &'a [ReviewRecord],
    t: &'a Thresholds,
    tz: &'a FixedOffset,
    now: DateTime<Utc>,
}

fn language_stats(language: &str, entries: &[&Entry], ctx: &Ctx) -> LanguageStats {
    let Ctx { settings, prefs, reviews, t, tz, now } = *ctx;
    let rated: Vec<RatedText> =
        entries.iter().filter(|e| e.word_count >= t.min_text_words).map(|e| RatedText { entry: e, words: e.word_count }).collect();

    // Totals.
    let week = |d: DateTime<Utc>| {
        let w = local_day(d, tz).iso_week();
        (w.year(), w.week())
    };
    let active_from = now - Duration::weeks(12);
    let active_weeks: BTreeSet<_> = entries.iter().filter(|e| e.created_at > active_from).map(|e| week(e.created_at)).collect();
    let totals = Totals {
        texts: entries.len() as u32,
        words: entries.iter().map(|e| e.word_count).sum(),
        rated_words: rated.iter().map(|r| r.words).sum(),
        active_weeks_12: active_weeks.len() as u32,
        first_at: entries.first().map(|e| e.created_at),
        last_at: entries.last().map(|e| e.created_at),
    };

    let error_rate = rate_trend(&rated, t, |e| e.mistakes.len() as u32);
    let global_error_rate = rate_trend(&rated, t, |e| e.mistakes.iter().filter(|m| m.impedes_meaning).count() as u32);

    // By tone.
    let mut tones: BTreeMap<String, (String, Vec<RatedText>)> = BTreeMap::new();
    for r in &rated {
        tones.entry(r.entry.tone.id.clone()).or_insert_with(|| (r.entry.tone.name.clone(), vec![])).1.push(RatedText { entry: r.entry, words: r.words });
    }
    let mut by_tone: Vec<ToneRate> = tones
        .into_iter()
        .map(|(tone_id, (tone_name, texts))| ToneRate {
            texts: texts.len() as u32,
            words: texts.iter().map(|x| x.words).sum(),
            error_rate: rate_trend(&texts, t, |e| e.mistakes.len() as u32),
            tone_id,
            tone_name,
        })
        .collect();
    by_tone.sort_by_key(|t| std::cmp::Reverse(t.words));

    // Areas and categories (shares from the two windows).
    let window_errors = error_rate.recent_errors + error_rate.prior_errors;
    let mut areas = Vec::new();
    for area in [ErrorArea::Grammar, ErrorArea::Lexis, ErrorArea::Orthography, ErrorArea::Register, ErrorArea::Discourse, ErrorArea::Other] {
        let rate = rate_trend(&rated, t, |e| e.mistakes.iter().filter(|m| m.category.area() == area).count() as u32);
        let area_errors = rate.recent_errors + rate.prior_errors;
        if area_errors == 0 {
            continue;
        }
        let mut categories: Vec<CategoryStats> = ErrorCategory::ALL
            .iter()
            .filter(|c| c.area() == area)
            .filter_map(|c| {
                let rate = rate_trend(&rated, t, |e| e.mistakes.iter().filter(|m| m.category == *c).count() as u32);
                let n = rate.recent_errors + rate.prior_errors;
                (n > 0).then(|| CategoryStats { category: *c, kind: c.kind(), share: share(n, window_errors), rate })
            })
            .collect();
        categories.sort_by(|a, b| b.share.total_cmp(&a.share));
        areas.push(AreaStats { area, share: share(area_errors, window_errors), rate, categories });
    }
    areas.sort_by(|a, b| b.share.total_cmp(&a.share));

    // Patterns: walk every entry in order (short texts count here too).
    let mut lifecycles: HashMap<String, Lifecycle> = HashMap::new();
    let mut meta: HashMap<String, (String, ErrorCategory)> = HashMap::new();
    let mut examples: HashMap<String, Vec<PatternExample>> = HashMap::new();
    let mut impeding: HashMap<String, (u32, u32)> = HashMap::new();
    for e in entries {
        let mut errors_here: BTreeMap<&str, u32> = BTreeMap::new();
        for m in &e.mistakes {
            *errors_here.entry(m.pattern_key.as_str()).or_default() += 1;
            meta.entry(m.pattern_key.clone()).or_insert((m.pattern.clone(), m.category));
            let imp = impeding.entry(m.pattern_key.clone()).or_default();
            imp.0 += u32::from(m.impedes_meaning);
            imp.1 += 1;
            let sentence = match m.span {
                Some(span) => sentence_around(&e.input, utf16_to_byte(&e.input, span.start), utf16_to_byte(&e.input, span.end)),
                None => m.original.clone(),
            };
            examples.entry(m.pattern_key.clone()).or_default().push(PatternExample {
                entry_id: e.id.clone(),
                at: e.created_at,
                original: m.original.clone(),
                correction: m.correction.clone(),
                sentence,
                explanation: m.explanation.clone(),
            });
        }
        let keys: BTreeSet<&str> = errors_here.keys().copied().chain(e.watch.iter().map(|w| w.key.as_str())).collect();
        for key in keys {
            let opp = e.watch.iter().find(|w| w.key == key).map(|w| (w.correct, w.errors));
            let errors = errors_here.get(key).copied().unwrap_or(0);
            lifecycles.entry(key.to_string()).or_insert_with(|| Lifecycle::new(t, *tz)).on_text(e.created_at, &e.id, errors, opp);
        }
    }

    let thirty = now - Duration::days(30);
    let mut patterns: Vec<PatternStats> = Vec::new();
    let mut mastered_recently = Vec::new();
    for (key, mut lc) in lifecycles {
        lc.tick(now);
        let Some(state) = lc.state else { continue };
        let Some((label, category)) = meta.get(&key).cloned() else { continue };
        if let Some(at) = lc.mastered_at.filter(|at| state == PatternState::Mastered && *at >= thirty) {
            mastered_recently.push(MasteredItem { key: key.clone(), label: label.clone(), mastered_at: at });
        }
        if state == PatternState::Archived {
            continue;
        }
        let texts: BTreeSet<&String> = lc.errors.iter().map(|(_, id)| id).collect();
        let days: BTreeSet<NaiveDate> = lc.errors.iter().map(|(at, _)| local_day(*at, tz)).collect();
        let days_30: BTreeSet<NaiveDate> = lc.errors.iter().filter(|(at, _)| *at >= thirty).map(|(at, _)| local_day(*at, tz)).collect();
        let (imp, total) = impeding.get(&key).copied().unwrap_or((0, 1));
        let mut ex = examples.remove(&key).unwrap_or_default();
        ex.reverse();
        ex.truncate(3);
        let pooled = lc.pooled_accuracy();
        let reminder = matches!(state, PatternState::Recurring | PatternState::Improving | PatternState::Relapsed)
            .then(|| reminder(&key, &lc.errors, &lc.all_opps, reviews, t, now))
            .flatten();
        patterns.push(PatternStats {
            label,
            category,
            kind: category.kind(),
            state,
            since: lc.since,
            first_seen: lc.errors.first().map(|e| e.0).unwrap_or(lc.since),
            last_error_at: lc.errors.last().map(|e| e.0),
            texts_with_error: texts.len() as u32,
            days_with_error: days.len() as u32,
            days_with_error_30d: days_30.len() as u32,
            impedes_meaning_share: share(imp, total),
            recent_accuracy: pooled.map(|(acc, _)| (acc * 1000.0).round() / 1000.0),
            opportunities_recent: pooled.map_or(0, |(_, n)| n),
            examples: ex,
            reminder,
            on_watch_list: false,
            pinned: prefs.pinned.contains(&key),
            dismissed: prefs.dismissed.contains(&key),
            key,
        });
    }

    // Watch list (§4.3): pinned first, then relapsed, improving, recurring (by days in the last
    // 30 days, then by how often they impeded meaning).
    let rank = |p: &PatternStats| match p.state {
        PatternState::Relapsed => 0,
        PatternState::Improving => 1,
        PatternState::Recurring => 2,
        _ => 9,
    };
    let mut candidates: Vec<&PatternStats> = patterns
        .iter()
        .filter(|p| !prefs.dismissed.contains(&p.key) && !p.pinned && rank(p) < 9)
        .collect();
    candidates.sort_by(|a, b| {
        rank(a)
            .cmp(&rank(b))
            .then(b.days_with_error_30d.cmp(&a.days_with_error_30d))
            .then(b.impedes_meaning_share.total_cmp(&a.impedes_meaning_share))
            .then(a.key.cmp(&b.key))
    });
    let mut watch_list: Vec<String> = patterns.iter().filter(|p| p.pinned).map(|p| p.key.clone()).collect();
    for c in candidates {
        if watch_list.len() >= t.watch_list_size as usize {
            break;
        }
        watch_list.push(c.key.clone());
    }
    for p in &mut patterns {
        p.on_watch_list = watch_list.contains(&p.key);
        if !p.on_watch_list {
            p.reminder = None;
        }
    }
    let state_order = |s: PatternState| match s {
        PatternState::Relapsed => 0,
        PatternState::Recurring => 1,
        PatternState::Improving => 2,
        PatternState::New => 3,
        PatternState::Mastered => 4,
        PatternState::Dormant => 5,
        PatternState::Archived => 6,
    };
    patterns.sort_by(|a, b| {
        let wa = watch_list.iter().position(|k| *k == a.key).unwrap_or(usize::MAX);
        let wb = watch_list.iter().position(|k| *k == b.key).unwrap_or(usize::MAX);
        wa.cmp(&wb).then(state_order(a.state).cmp(&state_order(b.state))).then(b.last_error_at.cmp(&a.last_error_at))
    });
    mastered_recently.sort_by_key(|m| std::cmp::Reverse(m.mastered_at));

    LanguageStats {
        language: language.to_string(),
        language_name: settings.language_name(language),
        totals,
        error_rate,
        global_error_rate,
        by_tone,
        areas,
        patterns,
        watch_list,
        mastered_recently,
        suggestions: suggestion_stats(entries, t, tz, now),
    }
}

/// Computes all stats from scratch. `entries` may be in any order.
/// `tz`: the learner's UTC offset, so "distinct days" and weeks are their days.
pub fn compute_stats(
    entries: &[Entry],
    settings: &Settings,
    prefs: &PatternPrefs,
    reviews: &[ReviewRecord],
    tz: FixedOffset,
    now: DateTime<Utc>,
) -> Stats {
    let t = Thresholds::default();
    let ctx = Ctx { settings, prefs, reviews, t: &t, tz: &tz, now };
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
    let mut by_lang: BTreeMap<&str, Vec<&Entry>> = BTreeMap::new();
    for e in &sorted {
        by_lang.entry(e.language.as_str()).or_default().push(e);
    }
    // Configured languages first, in settings order, then anything else found in history.
    let mut order: Vec<&str> = settings.languages.iter().map(|l| l.code.as_str()).filter(|c| by_lang.contains_key(c)).collect();
    order.extend(by_lang.keys().filter(|k| !order.contains(k)).copied().collect::<Vec<_>>());
    let languages = order
        .into_iter()
        .map(|lang| language_stats(lang, &by_lang[lang], &ctx))
        .collect();
    Stats {
        schema_version: SCHEMA_VERSION,
        generated_at: now,
        entry_count: entries.len() as u64,
        data_version: 0,
        thresholds: t,
        languages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisson_test_matches_known_values() {
        // Same rate → p = 1.
        assert!((poisson_rate_test(10, 1000, 10, 1000) - 1.0).abs() < 1e-9);
        // 5 vs 20 errors over equal words: binomial(25, .5) two-sided tail ≈ 0.0041.
        let p = poisson_rate_test(5, 1000, 20, 1000);
        assert!((p - 0.004_077).abs() < 1e-4, "{p}");
        // Small numbers are never significant.
        assert!(poisson_rate_test(1, 500, 3, 500) > 0.05);
    }

    #[test]
    fn ln_gamma_is_accurate() {
        assert!((ln_gamma(1.0)).abs() < 1e-12);
        assert!((ln_gamma(5.0) - 24f64.ln()).abs() < 1e-12);
        assert!((ln_gamma(0.5) - std::f64::consts::PI.sqrt().ln()).abs() < 1e-12);
    }

    #[test]
    fn sentence_extraction() {
        let t = "First one. Yesterday I go home! Third.";
        let s = t.find("I go").unwrap();
        assert_eq!(sentence_around(t, s, s + 4), "Yesterday I go home!");
    }
}

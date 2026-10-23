//! `Core`: the one object the desktop shell talks to.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset, Local, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::check::run_check;
use crate::error::{Error, Result};
use crate::export::{ExportBundle, write_export};
use crate::insights::{Insights, generate_insights};
use crate::llm::{LlmClient, ModelInfo, OutputMode, http_client};
use crate::model::Entry;
use crate::prompt::{CheckContext, KnownPattern, MAX_KNOWN_PATTERNS, WatchedPattern};
use crate::secrets::SecretStore;
use crate::settings::{ProviderConfig, Settings};
use crate::stats::{PatternPrefs, PatternState, ReviewRecord, Stats, compute_stats};
use crate::store::Store;
use crate::tone::Tone;

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CheckRequest {
    pub text: String,
    pub tone_id: String,
    /// `None` or "auto" = detect.
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub id: String,
    pub has_key: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub entries: Vec<Entry>,
    /// Pass back as `before` to get the next page; `None` when there is no more.
    pub next_before: Option<DateTime<Utc>>,
}

/// What the background worker did on an idle tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IdleOutcome {
    pub stats_updated: bool,
    pub insights_updated: bool,
}

pub struct Core {
    store: Store,
    secrets: Box<dyn SecretStore>,
    http: reqwest::Client,
    last_activity: Mutex<Instant>,
    /// Output mode that worked per "provider/model", so a fallback isn't retried on every check.
    modes: Mutex<HashMap<String, OutputMode>>,
    /// Set while an insights request is running, so idle ticks don't start a second one.
    insights_busy: AtomicBool,
    /// Checks in progress; background model calls wait until there are none.
    checks_in_flight: AtomicUsize,
}

/// Resets a flag / decrements a counter when dropped, so a failed or cancelled request can't
/// leave it stuck.
struct Release<'a>(&'a (dyn Fn() + Sync));

impl Drop for Release<'_> {
    fn drop(&mut self) {
        (self.0)()
    }
}

/// The learner's UTC offset right now (stats count days in local time).
fn local_offset() -> FixedOffset {
    *Local::now().offset()
}

const KEY_STATS: &str = "stats";
const KEY_INSIGHTS: &str = "insights";
const KEY_PREFS: &str = "pattern_prefs";
const KEY_REVIEWS: &str = "reviews";

impl Core {
    pub fn open(data_dir: &Path, secrets: Box<dyn SecretStore>) -> Result<Core> {
        Core::with_store(Store::open(&data_dir.join("refine.sqlite3"))?, secrets)
    }

    pub fn with_store(store: Store, secrets: Box<dyn SecretStore>) -> Result<Core> {
        Ok(Core {
            store,
            secrets,
            http: http_client(),
            last_activity: Mutex::new(Instant::now()),
            modes: Mutex::new(HashMap::new()),
            insights_busy: AtomicBool::new(false),
            checks_in_flight: AtomicUsize::new(0),
        })
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Marks the user as active (typing, clicking). Background work waits for idle time.
    pub fn touch(&self) {
        *self.last_activity.lock().unwrap() = Instant::now();
    }

    pub fn idle_for(&self) -> Duration {
        self.last_activity.lock().unwrap().elapsed()
    }

    // ---- settings & providers ------------------------------------------------------------

    pub fn settings(&self) -> Result<Settings> {
        self.store.settings()
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<Settings> {
        self.store.save_settings(settings)?;
        Ok(settings.clone())
    }

    fn key_account(provider_id: &str) -> String {
        format!("provider:{provider_id}")
    }

    pub fn set_api_key(&self, provider_id: &str, key: &str) -> Result<()> {
        let key = key.trim();
        if key.is_empty() {
            self.secrets.delete(&Core::key_account(provider_id))
        } else {
            self.secrets.set(&Core::key_account(provider_id), key)
        }
    }

    pub fn provider_statuses(&self) -> Result<Vec<ProviderStatus>> {
        self.settings()?
            .providers
            .iter()
            .map(|p| Ok(ProviderStatus { id: p.id.clone(), has_key: self.secrets.get(&Core::key_account(&p.id))?.is_some() }))
            .collect()
    }

    fn client_for(&self, provider: &ProviderConfig) -> Result<LlmClient> {
        let key = self.secrets.get(&Core::key_account(&provider.id))?;
        LlmClient::new(self.http.clone(), provider.clone(), key)
    }

    fn active_client(&self, settings: &Settings) -> Result<LlmClient> {
        let provider = settings.active_provider().ok_or_else(|| Error::Invalid("No provider selected.".into()))?;
        self.client_for(provider)
    }

    /// Lists models for a provider as configured in `provider` (may be unsaved edits).
    pub async fn list_models(&self, provider: &ProviderConfig) -> Result<Vec<ModelInfo>> {
        let mut provider = provider.clone();
        // Listing doesn't need a key on OpenRouter; don't block the picker on it.
        provider.needs_key = false;
        self.client_for(&provider)?.list_models().await
    }

    // ---- tones ---------------------------------------------------------------------------

    pub fn tones(&self) -> Result<Vec<Tone>> {
        self.store.tones()
    }

    pub fn save_tone(&self, tone: &Tone) -> Result<Tone> {
        let mut tone = tone.clone();
        if tone.id.is_empty() {
            tone.id = format!("custom-{}", uuid::Uuid::now_v7());
            tone.builtin = false;
        }
        self.store.save_tone(&tone)?;
        Ok(tone)
    }

    pub fn delete_tone(&self, id: &str) -> Result<()> {
        self.store.delete_tone(id)?;
        let mut settings = self.settings()?;
        if settings.tone_id == id {
            settings.tone_id = self.store.tones()?.first().map(|t| t.id.clone()).unwrap_or_default();
            self.store.save_settings(&settings)?;
        }
        Ok(())
    }

    // ---- checking ------------------------------------------------------------------------

    pub async fn check(&self, req: &CheckRequest) -> Result<Entry> {
        self.touch();
        self.checks_in_flight.fetch_add(1, Ordering::SeqCst);
        let release = || {
            self.checks_in_flight.fetch_sub(1, Ordering::SeqCst);
        };
        let _in_flight = Release(&release);
        let settings = self.settings()?;
        let tone = self
            .store
            .tone(&req.tone_id)?
            .or_else(|| self.store.tones().ok().and_then(|t| t.into_iter().next()))
            .ok_or_else(|| Error::Invalid("Create a tone first.".into()))?;
        let language = req.language.as_deref().filter(|l| *l != "auto" && !l.is_empty());
        if let Some(code) = language
            && settings.language(code).is_none()
        {
            return Err(Error::Invalid(format!("Add \"{code}\" to your languages first.")));
        }
        let client = self.active_client(&settings)?;
        let mode_key = format!("{}/{}", client.provider().id, client.provider().model);
        let mode = self.modes.lock().unwrap().get(&mode_key).copied().unwrap_or(OutputMode::JsonSchema);

        let (known, watched) = self.pattern_context()?;
        let ctx = CheckContext { settings: &settings, tone: &tone, language, known_patterns: &known, watched: &watched };
        let outcome = run_check(&client, &ctx, &req.text, mode).await?;
        self.modes.lock().unwrap().insert(mode_key, outcome.mode);
        self.store.insert_entry(&outcome.entry)?;
        self.touch();
        Ok(outcome.entry)
    }

    // ---- history -------------------------------------------------------------------------

    pub fn history(&self, before: Option<DateTime<Utc>>, limit: usize, query: Option<&str>) -> Result<HistoryPage> {
        let limit = limit.clamp(1, 200);
        let query = query.map(crate::text::normalize).filter(|q| !q.is_empty());
        let Some(q) = query else {
            let entries = self.store.entries(before, limit + 1)?;
            return Ok(page(entries, limit));
        };
        // Search: scan newest-first and keep matches. Fine for one person's history.
        let mut matched = Vec::new();
        let mut cursor = before;
        loop {
            let batch = self.store.entries(cursor, 500)?;
            let done = batch.len() < 500;
            cursor = batch.last().map(|e| e.created_at);
            for e in batch {
                let hay = crate::text::normalize(&format!("{}\n{}", e.input, e.fixed_text));
                if hay.contains(&q) {
                    matched.push(e);
                    if matched.len() > limit {
                        return Ok(page(matched, limit));
                    }
                }
            }
            if done {
                return Ok(page(matched, limit));
            }
        }
    }

    pub fn entry(&self, id: &str) -> Result<Entry> {
        self.store.entry(id)
    }

    pub fn delete_entry(&self, id: &str) -> Result<()> {
        self.store.delete_entry(id)
    }

    // ---- stats ---------------------------------------------------------------------------

    /// Fresh stats computed from all entries (cheap: no LLM), also cached for the export.
    pub fn recompute_stats(&self) -> Result<Stats> {
        let settings = self.settings()?;
        let version = self.store.data_version()?;
        let mut stats =
            compute_stats(&self.store.all_entries()?, &settings, &self.pattern_prefs()?, &self.reviews()?, local_offset(), Utc::now());
        stats.data_version = version;
        self.store.set_meta(KEY_STATS, &stats)?;
        Ok(stats)
    }

    /// The cached stats, if nothing changed since and it's still the same day (windows and
    /// "last 30 days" move with the date).
    fn cached_stats(&self) -> Result<Option<Stats>> {
        // An unreadable cache (e.g. from an older version) is just recomputed.
        let Ok(Some(stats)) = self.store.get_meta::<Stats>(KEY_STATS) else { return Ok(None) };
        let today = |d: DateTime<Utc>| d.with_timezone(&local_offset()).date_naive();
        let current = stats.data_version == self.store.data_version()? && today(stats.generated_at) == today(Utc::now());
        Ok(current.then_some(stats))
    }

    /// Cached stats if they're current, otherwise recomputed.
    pub fn stats(&self) -> Result<Stats> {
        match self.cached_stats()? {
            Some(stats) => Ok(stats),
            None => self.recompute_stats(),
        }
    }

    /// Known pattern labels (for reuse) and the watch list (for opportunity counts), from stats.
    fn pattern_context(&self) -> Result<(Vec<KnownPattern>, Vec<WatchedPattern>)> {
        let stats = self.stats()?;
        let mut known = Vec::new();
        let mut watched = Vec::new();
        for l in &stats.languages {
            let mut patterns: Vec<_> = l.patterns.iter().filter(|p| p.state != PatternState::Archived).collect();
            patterns.sort_by_key(|p| std::cmp::Reverse(p.last_error_at));
            known.extend(patterns.iter().take(MAX_KNOWN_PATTERNS).map(|p| KnownPattern {
                language: l.language.clone(),
                category: p.category,
                label: p.label.clone(),
            }));
            for p in l.patterns.iter().filter(|p| p.on_watch_list) {
                // Same label in two categories: the model can't tell them apart; watch the first.
                if watched.iter().any(|w: &WatchedPattern| w.language == l.language && w.label == p.label) {
                    continue;
                }
                watched.push(WatchedPattern { key: p.key.clone(), language: l.language.clone(), label: p.label.clone() });
            }
        }
        Ok((known, watched))
    }

    pub fn pattern_prefs(&self) -> Result<PatternPrefs> {
        Ok(self.store.get_meta(KEY_PREFS)?.unwrap_or_default())
    }

    /// Pin (`Some(true)`), dismiss (`Some(false)`) or reset (`None`) a pattern on the watch list.
    pub fn set_pattern_pref(&self, key: &str, pinned: Option<bool>) -> Result<Stats> {
        let mut prefs = self.pattern_prefs()?;
        prefs.pinned.retain(|k| k != key);
        prefs.dismissed.retain(|k| k != key);
        match pinned {
            Some(true) => prefs.pinned.push(key.to_string()),
            Some(false) => prefs.dismissed.push(key.to_string()),
            None => {}
        }
        self.store.set_meta(KEY_PREFS, &prefs)?;
        self.recompute_stats()
    }

    pub fn reviews(&self) -> Result<Vec<ReviewRecord>> {
        Ok(self.store.get_meta(KEY_REVIEWS)?.unwrap_or_default())
    }

    /// Records a self-graded "fix your own sentence" review; moves the reminder ladder.
    pub fn record_review(&self, key: &str, passed: bool) -> Result<Stats> {
        let mut reviews = self.reviews()?;
        reviews.push(ReviewRecord { pattern_key: key.to_string(), at: Utc::now(), passed });
        self.store.set_meta(KEY_REVIEWS, &reviews)?;
        self.recompute_stats()
    }

    pub fn insights(&self) -> Result<Option<Insights>> {
        self.store.get_meta(KEY_INSIGHTS)
    }

    /// Asks the model for the written summary of the current stats.
    pub async fn refresh_insights(&self) -> Result<Insights> {
        if self.insights_busy.swap(true, Ordering::SeqCst) {
            return Err(Error::Invalid("A summary is already being written.".into()));
        }
        let release = || self.insights_busy.store(false, Ordering::SeqCst);
        let _busy = Release(&release);
        let settings = self.settings()?;
        let stats = self.recompute_stats()?;
        let client = self.active_client(&settings)?;
        let insights = generate_insights(&client, &settings, &stats, Utc::now()).await?;
        self.store.set_meta(KEY_INSIGHTS, &insights)?;
        Ok(insights)
    }

    /// Background work while the user is away: refresh stats, and the written summary when the
    /// policy allows (enough new texts and time since the last one).
    pub async fn idle_tick(&self) -> Result<IdleOutcome> {
        let settings = self.settings()?;
        let mut out = IdleOutcome::default();
        let idle = Duration::from_secs(u64::from(settings.idle_minutes) * 60);
        // A slow local model can take minutes per check; don't compete with it.
        if self.idle_for() < idle || self.checks_in_flight.load(Ordering::SeqCst) > 0 {
            return Ok(out);
        }
        let stats = match self.cached_stats()? {
            Some(s) => s,
            None => {
                out.stats_updated = true;
                self.recompute_stats()?
            }
        };

        let policy = &settings.insights;
        if !policy.auto || stats.entry_count == 0 {
            return Ok(out);
        }
        let last = self.insights()?;
        let due = match &last {
            None => stats.entry_count >= u64::from(policy.min_new_entries),
            Some(i) => {
                let hours = (Utc::now() - i.generated_at).num_hours();
                stats.entry_count.saturating_sub(i.entry_count) >= u64::from(policy.min_new_entries)
                    && hours >= i64::from(policy.min_hours_between)
            }
        };
        if due && self.active_client(&settings).is_ok() {
            match self.refresh_insights().await {
                Ok(_) => out.insights_updated = true,
                Err(e) => tracing::warn!(error = %e, "idle insights failed"),
            }
        }
        Ok(out)
    }

    // ---- export --------------------------------------------------------------------------

    pub fn export_bundle(&self) -> Result<ExportBundle> {
        Ok(ExportBundle::new(
            self.settings()?,
            self.tones()?,
            self.store.all_entries()?,
            Some(self.recompute_stats()?),
            self.insights()?,
        ))
    }

    /// Writes everything to one JSON file in `dir` and returns its path.
    pub fn export_to(&self, dir: &Path) -> Result<PathBuf> {
        write_export(&self.export_bundle()?, dir)
    }
}

fn page(mut entries: Vec<Entry>, limit: usize) -> HistoryPage {
    let more = entries.len() > limit;
    entries.truncate(limit);
    let next_before = if more { entries.last().map(|e| e.created_at) } else { None };
    HistoryPage { entries, next_before }
}

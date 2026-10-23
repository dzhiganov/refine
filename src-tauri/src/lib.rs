//! Tauri shell: commands are thin wrappers over `refine_core::app::Core`.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use refine_core::app::{CheckRequest, Core, HistoryPage, ProviderStatus};
use refine_core::insights::Insights;
use refine_core::llm::ModelInfo;
use refine_core::model::Entry;
use refine_core::settings::{ProviderConfig, Settings};
use refine_core::stats::Stats;
use refine_core::taxonomy::{CategoryInfo, catalog};
use refine_core::tone::Tone;
use refine_core::error::AppErrorKind;
use refine_core::{AppError, secrets};
use serde::Serialize;
use tauri::{Emitter, Manager, State};

type CoreState<'a> = State<'a, Arc<Core>>;
type CmdResult<T> = Result<T, AppError>;

/// Everything the UI needs on start, in one round trip.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    app_version: &'static str,
    platform: &'static str,
    settings: Settings,
    tones: Vec<Tone>,
    providers: Vec<ProviderStatus>,
    categories: Vec<CategoryInfo>,
}

/// Runs blocking core work (SQLite, keychain, stats) on a worker thread, never on the UI thread.
async fn blocking<T, F>(core: &Arc<Core>, f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce(&Core) -> refine_core::Result<T> + Send + 'static,
{
    let core = core.clone();
    tauri::async_runtime::spawn_blocking(move || f(&core))
        .await
        .map_err(|e| AppError { kind: AppErrorKind::Internal, message: e.to_string() })?
        .map_err(Into::into)
}

#[tauri::command]
async fn bootstrap(core: CoreState<'_>) -> CmdResult<Bootstrap> {
    blocking(&core, |c| {
        Ok(Bootstrap {
            app_version: env!("CARGO_PKG_VERSION"),
            platform: std::env::consts::OS,
            settings: c.settings()?,
            tones: c.tones()?,
            providers: c.provider_statuses()?,
            categories: catalog(),
        })
    })
    .await
}

#[tauri::command]
async fn save_settings(core: CoreState<'_>, settings: Settings) -> CmdResult<Settings> {
    blocking(&core, move |c| c.save_settings(&settings)).await
}

#[tauri::command]
async fn set_api_key(core: CoreState<'_>, provider_id: String, key: String) -> CmdResult<Vec<ProviderStatus>> {
    blocking(&core, move |c| {
        c.set_api_key(&provider_id, &key)?;
        c.provider_statuses()
    })
    .await
}

#[tauri::command]
async fn list_models(core: CoreState<'_>, provider: ProviderConfig) -> CmdResult<Vec<ModelInfo>> {
    Ok(core.list_models(&provider).await?)
}

#[tauri::command]
async fn save_tone(core: CoreState<'_>, tone: Tone) -> CmdResult<Tone> {
    blocking(&core, move |c| c.save_tone(&tone)).await
}

#[tauri::command]
async fn delete_tone(core: CoreState<'_>, id: String) -> CmdResult<Vec<Tone>> {
    blocking(&core, move |c| {
        c.delete_tone(&id)?;
        c.tones()
    })
    .await
}

#[tauri::command]
async fn check_text(core: CoreState<'_>, request: CheckRequest) -> CmdResult<Entry> {
    Ok(core.check(&request).await?)
}

#[tauri::command]
async fn history(
    core: CoreState<'_>,
    before: Option<DateTime<Utc>>,
    limit: Option<usize>,
    query: Option<String>,
) -> CmdResult<HistoryPage> {
    blocking(&core, move |c| c.history(before, limit.unwrap_or(50), query.as_deref())).await
}

#[tauri::command]
async fn get_entry(core: CoreState<'_>, id: String) -> CmdResult<Entry> {
    blocking(&core, move |c| c.entry(&id)).await
}

#[tauri::command]
async fn delete_entry(core: CoreState<'_>, id: String) -> CmdResult<()> {
    blocking(&core, move |c| c.delete_entry(&id)).await
}

#[tauri::command]
async fn get_stats(core: CoreState<'_>) -> CmdResult<Stats> {
    blocking(&core, |c| c.stats()).await
}

#[tauri::command]
async fn get_insights(core: CoreState<'_>) -> CmdResult<Option<Insights>> {
    blocking(&core, |c| c.insights()).await
}

#[tauri::command]
async fn refresh_insights(core: CoreState<'_>) -> CmdResult<Insights> {
    Ok(core.refresh_insights().await?)
}

/// Pin (`true`), dismiss (`false`) or reset (`null`) a pattern on the watch list.
#[tauri::command]
async fn set_pattern_pref(core: CoreState<'_>, key: String, pinned: Option<bool>) -> CmdResult<Stats> {
    blocking(&core, move |c| c.set_pattern_pref(&key, pinned)).await
}

/// A self-graded "fix your own sentence" review.
#[tauri::command]
async fn record_review(core: CoreState<'_>, key: String, passed: bool) -> CmdResult<Stats> {
    blocking(&core, move |c| c.record_review(&key, passed)).await
}

/// Writes the export to the Downloads folder and returns the file path.
#[tauri::command]
async fn export_history(app: tauri::AppHandle, core: CoreState<'_>) -> CmdResult<String> {
    let dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().document_dir())
        .map_err(|e| AppError::from(refine_core::Error::Invalid(format!("No Downloads folder: {e}"))))?;
    blocking(&core, move |c| Ok(c.export_to(&dir)?.to_string_lossy().into_owned())).await
}

#[tauri::command]
fn touch(core: CoreState) {
    core.touch();
}

/// Checks every 30 s whether the user is idle and, if so, refreshes stats/insights in the
/// background. The UI listens for `stats-updated` / `insights-updated`.
fn spawn_idle_worker(app: tauri::AppHandle, core: Arc<Core>) {
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        loop {
            tick.tick().await;
            match core.idle_tick().await {
                Ok(out) => {
                    if out.stats_updated {
                        let _ = app.emit("stats-updated", ());
                    }
                    if out.insights_updated {
                        let _ = app.emit("insights-updated", ());
                    }
                }
                Err(e) => tracing::warn!(error = %e, "idle tick failed"),
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // REFINE_DATA_DIR: a separate data folder (tests, trying things out).
            let data_dir = match std::env::var_os("REFINE_DATA_DIR") {
                Some(dir) => std::path::PathBuf::from(dir),
                None => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&data_dir)?;
            tracing::info!(dir = %data_dir.display(), "data directory");
            let core = Arc::new(Core::open(&data_dir, secrets::platform_store(&data_dir))?);
            spawn_idle_worker(app.handle().clone(), core.clone());
            app.manage(core);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            save_settings,
            set_api_key,
            list_models,
            save_tone,
            delete_tone,
            check_text,
            history,
            get_entry,
            delete_entry,
            get_stats,
            get_insights,
            refresh_insights,
            set_pattern_pref,
            record_review,
            export_history,
            touch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Refine");
}

//! One-click export: everything in one versioned JSON file (no API keys).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::Result;
use crate::insights::Insights;
use crate::model::{Entry, SCHEMA_VERSION};
use crate::settings::Settings;
use crate::stats::Stats;
use crate::tone::Tone;

pub const EXPORT_FORMAT: &str = "refine.export";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    /// Always "refine.export".
    pub format: String,
    pub schema_version: u32,
    pub app_version: String,
    pub exported_at: DateTime<Utc>,
    pub settings: Settings,
    pub tones: Vec<Tone>,
    /// Oldest first.
    pub entries: Vec<Entry>,
    pub stats: Option<Stats>,
    pub insights: Option<Insights>,
}

impl ExportBundle {
    pub fn new(settings: Settings, tones: Vec<Tone>, entries: Vec<Entry>, stats: Option<Stats>, insights: Option<Insights>) -> Self {
        ExportBundle {
            format: EXPORT_FORMAT.into(),
            schema_version: SCHEMA_VERSION,
            app_version: env!("CARGO_PKG_VERSION").into(),
            exported_at: Utc::now(),
            settings,
            tones,
            entries,
            stats,
            insights,
        }
    }
}

/// Writes `refine-export-YYYY-MM-DD.json` (or `-2`, `-3`... if taken) into `dir`.
pub fn write_export(bundle: &ExportBundle, dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let stem = format!("refine-export-{}", bundle.exported_at.format("%Y-%m-%d"));
    let mut path = dir.join(format!("{stem}.json"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-{n}.json"));
        n += 1;
    }
    let tmp = path.with_extension("json.part");
    std::fs::write(&tmp, serde_json::to_vec_pretty(bundle)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

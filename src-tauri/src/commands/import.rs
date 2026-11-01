//! CSV/JSON import commands for the column-mapping screen.

use std::path::PathBuf;

use reader_csv_import::inspect::{self, Inspection, Preview};
use reader_csv_import::{Config, Format, Mapping};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::error::IpcResult;
use crate::state::AppState;

/// Ledger fields a column can map to, as used by the UI.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportMapping {
    pub ts: String,
    pub app: Option<String>,
    pub model: Option<String>,
    pub project: Option<String>,
    pub input_tokens: Option<String>,
    pub output_tokens: Option<String>,
    pub cache_read_tokens: Option<String>,
    pub cache_write_tokens: Option<String>,
    pub requests: Option<String>,
    pub service_tier: Option<String>,
}

impl From<Mapping> for ImportMapping {
    fn from(m: Mapping) -> Self {
        Self {
            ts: m.ts,
            app: m.app,
            model: m.model,
            project: m.project,
            input_tokens: m.input_tokens,
            output_tokens: m.output_tokens,
            cache_read_tokens: m.cache_read_tokens,
            cache_write_tokens: m.cache_write_tokens,
            requests: m.requests,
            service_tier: m.service_tier,
        }
    }
}

impl From<ImportMapping> for Mapping {
    fn from(m: ImportMapping) -> Self {
        let col = |c: Option<String>| c.filter(|s| !s.trim().is_empty());
        Self {
            ts: m.ts,
            app: col(m.app),
            model: col(m.model),
            project: col(m.project),
            input_tokens: col(m.input_tokens),
            output_tokens: col(m.output_tokens),
            cache_read_tokens: col(m.cache_read_tokens),
            cache_write_tokens: col(m.cache_write_tokens),
            requests: col(m.requests),
            service_tier: col(m.service_tier),
        }
    }
}

/// A chosen file's columns and a suggested mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportInspection {
    /// `csv` or `json`.
    pub format: String,
    pub columns: Vec<String>,
    pub sample: Vec<Vec<String>>,
    pub rows: u32,
    pub suggested: ImportMapping,
}

/// The file, format, mapping and fallback app the user chose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportSpec {
    pub path: String,
    /// `csv` or `json`.
    pub format: String,
    pub mapping: ImportMapping,
    pub default_app: String,
}

/// What an import would add.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub events: u32,
    pub skipped: u32,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

fn n(v: usize) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

fn fmt_name(f: Format) -> String {
    match f {
        Format::Csv => "csv",
        Format::Json => "json",
    }
    .into()
}

/// Logic callable without a Tauri runtime.
pub mod core {
    use super::*;

    pub fn to_config(spec: ImportSpec) -> IpcResult<Config> {
        let format = match spec.format.as_str() {
            "csv" => Format::Csv,
            "json" => Format::Json,
            other => {
                return Err(
                    ledger_core::Error::InvalidInput(format!("unknown format {other:?}")).into(),
                )
            }
        };
        let app = spec.default_app.trim();
        let mapped_app = spec
            .mapping
            .app
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty());
        if !mapped_app && app.is_empty() {
            return Err(ledger_core::Error::InvalidInput(
                "map an app column or enter an app name".into(),
            )
            .into());
        }
        Ok(Config {
            path: PathBuf::from(spec.path),
            format,
            mapping: spec.mapping.into(),
            default_app: app.to_owned(),
        })
    }

    pub fn inspect(path: &str) -> IpcResult<ImportInspection> {
        let Inspection {
            format,
            columns,
            sample,
            rows,
            suggested,
        } = inspect::inspect(std::path::Path::new(path))?;
        Ok(ImportInspection {
            format: fmt_name(format),
            columns,
            sample,
            rows: n(rows),
            suggested: suggested.into(),
        })
    }

    pub fn preview(spec: ImportSpec) -> IpcResult<ImportPreview> {
        let Preview {
            events,
            skipped,
            first_ts,
            last_ts,
            input_tokens,
            output_tokens,
        } = inspect::preview(&to_config(spec)?)?;
        Ok(ImportPreview {
            events: n(events),
            skipped: n(skipped),
            first_ts,
            last_ts,
            input_tokens,
            output_tokens,
        })
    }

    /// Adds the import as a source and reads it once. Returns the source id.
    pub fn add(state: &AppState, spec: ImportSpec) -> IpcResult<i64> {
        let cfg = to_config(spec)?;
        inspect::preview(&cfg)?;
        let label = cfg
            .path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| "import".into());
        let cand = ledger_core::reader::SourceCandidate {
            reader_id: reader_csv_import::READER_ID.into(),
            label,
            config: serde_json::to_value(&cfg).map_err(ledger_core::Error::from)?,
        };
        let id =
            ledger_core::sources::add(&state.db(), &cand, ledger_core::reader::Tier::Export)?.id;
        crate::live::sync_one(state, id)?;
        Ok(id)
    }
}

/// Lists a CSV/JSON export's columns and suggests a mapping.
#[tauri::command]
#[specta::specta]
pub fn inspect_import(path: String) -> IpcResult<ImportInspection> {
    core::inspect(&path)
}

/// Shows what an import would add, without storing anything.
#[tauri::command]
#[specta::specta]
pub fn preview_import(spec: ImportSpec) -> IpcResult<ImportPreview> {
    core::preview(spec)
}

/// Adds a CSV/JSON import source with the chosen mapping and reads it.
#[tauri::command]
#[specta::specta]
pub fn add_import(state: State<'_, AppState>, spec: ImportSpec) -> IpcResult<i64> {
    core::add(&state, spec)
}

//! Source commands: detect readers' data on this machine and manage sources.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;
use tauri::State;

use crate::error::{IpcError, IpcResult};
use crate::state::AppState;
use ledger_core::reader::{Health, SourceCandidate};

/// A source the user can enable, found by a reader.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub reader_id: String,
    pub label: String,
    pub config: Value,
    /// Whether an identical source is already configured.
    pub added: bool,
}

/// A configured source with its latest health.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub id: i64,
    pub reader_id: String,
    pub tier: i64,
    pub config: Value,
    pub enabled: bool,
    /// `unknown`, `healthy`, or `unhealthy`.
    pub health: String,
    pub error: Option<String>,
}

/// Logic callable without a Tauri runtime.
pub mod core {
    use super::*;

    pub fn list(state: &AppState) -> IpcResult<Vec<SourceInfo>> {
        let sources = ledger_core::sources::list(&state.db())?;
        let reg = state.registry();
        Ok(sources
            .into_iter()
            .map(|s| {
                let (health, error) = match reg.health(s.id) {
                    Health::Unknown => ("unknown", None),
                    Health::Healthy { .. } => ("healthy", None),
                    Health::Unhealthy { error } => ("unhealthy", Some(error)),
                };
                SourceInfo {
                    id: s.id,
                    reader_id: s.reader_id,
                    tier: s.tier,
                    config: s.config,
                    enabled: s.enabled,
                    health: health.into(),
                    error,
                }
            })
            .collect())
    }

    pub fn detect(state: &AppState) -> IpcResult<Vec<Candidate>> {
        let found = state.registry().detect_all();
        let existing = ledger_core::sources::list(&state.db())?;
        Ok(found
            .into_iter()
            .map(|c| Candidate {
                added: existing
                    .iter()
                    .any(|s| s.reader_id == c.reader_id && s.config == c.config),
                reader_id: c.reader_id,
                label: c.label,
                config: c.config,
            })
            .collect())
    }

    pub fn add(state: &AppState, c: Candidate) -> IpcResult<i64> {
        let tier = state
            .registry()
            .get(&c.reader_id)
            .map(|r| r.tier())
            .ok_or_else(|| IpcError {
                code: "invalid_input".into(),
                message: format!("unknown reader {:?}", c.reader_id),
            })?;
        let cand = SourceCandidate {
            reader_id: c.reader_id,
            label: c.label,
            config: c.config,
        };
        Ok(ledger_core::sources::add(&state.db(), &cand, tier)?.id)
    }

    pub fn set_enabled(state: &AppState, id: i64, enabled: bool) -> IpcResult<()> {
        Ok(ledger_core::sources::set_enabled(&state.db(), id, enabled)?)
    }
}

/// Lists configured sources with their health.
#[tauri::command]
#[specta::specta]
pub fn list_sources(state: State<'_, AppState>) -> IpcResult<Vec<SourceInfo>> {
    core::list(&state)
}

/// Looks for AI apps' data on this machine.
#[tauri::command]
#[specta::specta]
pub fn detect_sources(state: State<'_, AppState>) -> IpcResult<Vec<Candidate>> {
    core::detect(&state)
}

/// Adds a detected (or user-built) source. Returns its id.
#[tauri::command]
#[specta::specta]
pub fn add_source(state: State<'_, AppState>, candidate: Candidate) -> IpcResult<i64> {
    core::add(&state, candidate)
}

/// Enables or disables a source.
#[tauri::command]
#[specta::specta]
pub fn set_source_enabled(state: State<'_, AppState>, id: i64, enabled: bool) -> IpcResult<()> {
    core::set_enabled(&state, id, enabled)
}

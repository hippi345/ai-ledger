//! Sessions commands (spec §D): the session list and one session's timeline.

use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::commands::aggregate::AggFilters;
use crate::error::IpcResult;
use crate::state::AppState;

/// One session's usage within the queried range.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub id: String,
    pub app: String,
    pub project: Option<String>,
    /// First and last event in the range (RFC 3339 UTC).
    pub first_ts: String,
    pub last_ts: String,
    pub events: u32,
    pub tokens: f64,
    /// None when none of its events has a price.
    pub cost_usd: Option<f64>,
    /// Models used, comma-separated.
    pub models: String,
}

/// A request (`tool` null) or a tool call in a session's timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TimelineItem {
    pub ts_utc: String,
    pub model: Option<String>,
    pub tokens: f64,
    pub cost_usd: Option<f64>,
    pub tool: Option<String>,
    pub mcp_server: Option<String>,
    pub duration_ms: Option<f64>,
    pub ok: bool,
}

/// Core logic (tests). Dates are UTC `YYYY-MM-DD`, `since <= day < until`.
pub fn list(
    state: &AppState,
    since: &str,
    until: &str,
    filters: &AggFilters,
) -> IpcResult<Vec<SessionRow>> {
    let rows = ledger_core::sessions::list(
        &state.db(),
        &format!("{since}T00:00:00Z"),
        &format!("{until}T00:00:00Z"),
        &ledger_core::aggregate::Filters {
            app: filters.app.clone(),
            model: filters.model.clone(),
            project: filters.project.clone(),
        },
    )?;
    Ok(rows
        .into_iter()
        .map(|s| SessionRow {
            id: s.id,
            app: s.app,
            project: s.project,
            first_ts: s.first_ts,
            last_ts: s.last_ts,
            events: u32::try_from(s.events).unwrap_or(u32::MAX),
            tokens: s.tokens as f64,
            cost_usd: s.cost_usd,
            models: s.models,
        })
        .collect())
}

/// Core logic (tests).
pub fn timeline(state: &AppState, id: &str) -> IpcResult<Vec<TimelineItem>> {
    Ok(ledger_core::sessions::timeline(&state.db(), id)?
        .into_iter()
        .map(|t| TimelineItem {
            ts_utc: t.ts_utc,
            model: t.model,
            tokens: t.tokens as f64,
            cost_usd: t.cost_usd,
            tool: t.tool,
            mcp_server: t.mcp_server,
            duration_ms: t.duration_ms.map(|v| v as f64),
            ok: t.ok,
        })
        .collect())
}

/// Sessions with usage in a date range, newest first (at most 500).
#[tauri::command]
#[specta::specta]
pub fn list_sessions(
    state: State<'_, AppState>,
    since: String,
    until: String,
    filters: AggFilters,
) -> IpcResult<Vec<SessionRow>> {
    list(&state, &since, &until, &filters)
}

/// One session's requests and tool calls in time order.
#[tauri::command]
#[specta::specta]
pub fn session_timeline(state: State<'_, AppState>, id: String) -> IpcResult<Vec<TimelineItem>> {
    timeline(&state, &id)
}

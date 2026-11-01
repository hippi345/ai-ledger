//! Activity heatmap command (spec §C): hour of day x day of week, UTC.

use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::error::IpcResult;
use crate::state::AppState;

/// One heatmap cell. `weekday` 0 = Monday, `hour` 0..=23 UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct HeatCell {
    pub weekday: u8,
    pub hour: u8,
    pub events: u32,
    pub tokens: f64,
}

/// Core logic (tests). Dates are UTC `YYYY-MM-DD`, `since <= day < until`.
pub fn run(state: &AppState, since: &str, until: &str) -> IpcResult<Vec<HeatCell>> {
    let cells = ledger_core::stats::heatmap(
        &state.db(),
        &format!("{since}T00:00:00Z"),
        &format!("{until}T00:00:00Z"),
    )?;
    Ok(cells
        .into_iter()
        .map(|c| HeatCell {
            weekday: c.weekday,
            hour: c.hour,
            events: u32::try_from(c.events).unwrap_or(u32::MAX),
            tokens: c.tokens as f64,
        })
        .collect())
}

/// Activity by weekday and hour for a date range.
#[tauri::command]
#[specta::specta]
pub fn activity_heatmap(
    state: State<'_, AppState>,
    since: String,
    until: String,
) -> IpcResult<Vec<HeatCell>> {
    run(&state, &since, &until)
}

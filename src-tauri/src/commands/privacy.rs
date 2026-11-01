//! Privacy commands (spec §8).

use tauri::State;

use crate::error::IpcResult;
use crate::state::AppState;

/// Core logic (tests).
pub fn hash_existing(state: &AppState) -> IpcResult<u32> {
    let n = ledger_core::privacy::hash_existing(&mut state.db())?;
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Irreversibly replaces stored project names with hashes. Returns how many names changed.
#[tauri::command]
#[specta::specta]
pub fn hash_existing_projects(state: State<'_, AppState>) -> IpcResult<u32> {
    hash_existing(&state)
}

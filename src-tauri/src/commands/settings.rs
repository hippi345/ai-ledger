//! Settings commands: thin wrappers over `ledger_core::settings`.

use serde::Serialize;
use serde_json::Value;
use specta::Type;
use tauri::State;

use crate::error::IpcResult;
use crate::state::AppState;

/// One stored setting.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct SettingEntry {
    pub key: String,
    pub value: Value,
}

/// Core logic, callable without a Tauri runtime (tests).
pub mod core {
    use super::*;

    pub fn get(state: &AppState, key: &str) -> IpcResult<Option<Value>> {
        Ok(ledger_core::settings::get_json(&state.db(), key)?)
    }

    pub fn set(state: &AppState, key: &str, value: &Value) -> IpcResult<()> {
        Ok(ledger_core::settings::set(&state.db(), key, value)?)
    }

    pub fn reset(state: &AppState, key: &str) -> IpcResult<bool> {
        Ok(ledger_core::settings::reset(&state.db(), key)?)
    }

    pub fn list(state: &AppState) -> IpcResult<Vec<SettingEntry>> {
        Ok(ledger_core::settings::all(&state.db())?
            .into_iter()
            .map(|(key, value)| SettingEntry { key, value })
            .collect())
    }
}

/// Returns a setting's stored JSON value, or null when unset.
#[tauri::command]
#[specta::specta]
pub fn get_setting(state: State<'_, AppState>, key: String) -> IpcResult<Option<Value>> {
    core::get(&state, &key)
}

/// Stores a setting.
#[tauri::command]
#[specta::specta]
pub fn set_setting(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    key: String,
    value: Value,
) -> IpcResult<()> {
    core::set(&state, &key, &value)?;
    crate::tray::sync_pause(&app, &key);
    Ok(())
}

/// Removes a setting so its default applies. Returns whether it was set.
#[tauri::command]
#[specta::specta]
pub fn reset_setting(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    key: String,
) -> IpcResult<bool> {
    let was_set = core::reset(&state, &key)?;
    crate::tray::sync_pause(&app, &key);
    Ok(was_set)
}

/// Lists all stored settings, sorted by key.
#[tauri::command]
#[specta::specta]
pub fn list_settings(state: State<'_, AppState>) -> IpcResult<Vec<SettingEntry>> {
    core::list(&state)
}

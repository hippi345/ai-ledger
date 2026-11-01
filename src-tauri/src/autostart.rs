//! Launch at login (spec item 32): opt-in, off by default.

use ledger_core::settings::{self, keys};
use tauri_plugin_autostart::ManagerExt;

use crate::error::IpcResult;
use crate::state::AppState;

/// Whether the user turned on launch at login.
pub fn wanted(state: &AppState) -> bool {
    settings::get_or(&state.db(), keys::LAUNCH_AT_LOGIN, false).unwrap_or(false)
}

/// Makes the OS login item match `on`, touching it only when it differs.
pub fn apply(app: &tauri::AppHandle, on: bool) -> Result<(), String> {
    let m = app.autolaunch();
    let current = m.is_enabled().unwrap_or(false);
    let r = match (on, current) {
        (true, false) => m.enable(),
        (false, true) => m.disable(),
        _ => Ok(()),
    };
    r.map_err(|e| e.to_string())
}

/// Turns launch at login on or off and saves the choice.
#[tauri::command]
#[specta::specta]
pub fn set_launch_at_login(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    on: bool,
) -> IpcResult<()> {
    settings::set(&state.db(), keys::LAUNCH_AT_LOGIN, &on)?;
    if let Err(e) = apply(&app, on) {
        tracing::warn!(error = %e, "could not update launch at login");
    }
    Ok(())
}

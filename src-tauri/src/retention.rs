//! Applies the retention setting: one pruning pass at launch, then daily.

use std::time::Duration;

use crate::state::AppState;

/// How often the background pass runs after the first one.
pub const EVERY: Duration = Duration::from_secs(24 * 60 * 60);

/// One pruning pass using `retention.raw_event_days` (0 = keep forever).
/// Returns how many events were deleted.
pub fn run_once(state: &AppState, now_utc: &str) -> ledger_core::Result<usize> {
    Ok(ledger_core::retention::run(&state.db(), now_utc)?.events_deleted)
}

/// Current time as RFC 3339 UTC, seconds precision.
pub fn now_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Spawns the daily retention thread. Errors are logged, never fatal.
pub fn spawn(handle: tauri::AppHandle) -> std::io::Result<()> {
    use tauri::Manager;
    std::thread::Builder::new()
        .name("ledger-retention".into())
        .spawn(move || loop {
            let st = handle.state::<AppState>();
            if let Err(e) = run_once(&st, &now_utc()) {
                tracing::warn!(error = %e, "retention pass failed");
            }
            std::thread::sleep(EVERY);
        })?;
    Ok(())
}

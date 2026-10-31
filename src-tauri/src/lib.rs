//! Tauri shell for AI Ledger.

pub mod ipc;
pub mod state;

use tauri::Manager;

/// Builds and runs the desktop app.
pub fn run() {
    let specta = ipc::builder();

    #[cfg(debug_assertions)]
    if let Err(e) = ipc::export_bindings(std::path::Path::new(ipc::BINDINGS_PATH)) {
        eprintln!("failed to export TypeScript bindings: {e}");
    }

    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            ledger_core::logging::init();
            specta.mount_events(app);
            let dir = app.path().app_data_dir()?;
            let state = state::AppState::open(&dir)?;
            tracing::info!(dir = %dir.display(), "ledger opened");
            app.manage(state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AI Ledger");
}

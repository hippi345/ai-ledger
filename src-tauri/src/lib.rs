//! Tauri shell for AI Ledger.

pub mod commands;
pub mod error;
pub mod ipc;
pub mod live;
pub mod state;
pub mod watcher;

use tauri::{Emitter, Manager};

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
            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("ledger-live".into())
                .spawn(move || {
                    let st = handle.state::<state::AppState>();
                    match live::prepare(&st) {
                        Ok((_watch, rx)) => {
                            live::run(&st, rx, ledger_core::debounce::DEFAULT_WINDOW, |id, s| {
                                tracing::info!(
                                    source = id,
                                    inserted = s.inserted,
                                    updated = s.updated,
                                    "live sync"
                                );
                                let ev = ipc::IngestedEvent::new(id, s);
                                if ev.changed() {
                                    let _ = handle.emit(ipc::INGESTED_EVENT, ev);
                                }
                            })
                        }
                        Err(e) => tracing::warn!(error = %e, "live updates unavailable"),
                    }
                })?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AI Ledger");
}

//! Tauri shell for AI Ledger.

pub mod autostart;
pub mod commands;
pub mod error;
pub mod ipc;
pub mod live;
pub mod proxy;
pub mod retention;
pub mod state;
pub mod tray;
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
        // Must be registered first: a second launch hands off to this one,
        // which shows and focuses its window, then exits.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        // Exports: a native save dialog, then write the bytes to the chosen path.
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(specta.invoke_handler())
        .on_window_event(|w, e| {
            // Minimize to tray: closing the window hides it while the tray is up.
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                if tray::active(w.app_handle()) {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
        })
        .setup(move |app| {
            ledger_core::logging::init();
            specta.mount_events(app);
            let dir = app.path().app_data_dir()?;
            let state = state::AppState::open(&dir)?;
            tracing::info!(dir = %dir.display(), "ledger opened");
            let launch = autostart::wanted(&state);
            app.manage(state);
            app.manage(proxy::ProxyState::default());
            app.manage(live::Live::default());
            // Off unless the user turned it on (proxy.enabled defaults to false).
            proxy::apply(app.handle());
            if let Err(e) = autostart::apply(app.handle(), launch) {
                tracing::warn!(error = %e, "could not sync launch at login");
            }
            if let Err(e) = tray::install(app.handle()) {
                tracing::warn!(error = %e, "tray unavailable; closing the window quits");
            }
            if let Err(e) = retention::spawn(app.handle().clone()) {
                tracing::warn!(error = %e, "retention thread not started");
            }
            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("ledger-live".into())
                .spawn(move || {
                    let st = handle.state::<state::AppState>();
                    let lv = handle.state::<live::Live>();
                    match lv.start(&st) {
                        Ok(rx) => {
                            live::run(&st, rx, ledger_core::debounce::DEFAULT_WINDOW, |id, s| {
                                tracing::info!(
                                    source = id,
                                    inserted = s.inserted,
                                    updated = s.updated,
                                    "live sync"
                                );
                                let ev = ipc::IngestedEvent::new(id, s);
                                if ev.changed() {
                                    tray::refresh(&handle);
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

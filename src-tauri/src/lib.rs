//! Tauri shell for AI Ledger.

pub mod ipc;

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
            specta.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AI Ledger");
}

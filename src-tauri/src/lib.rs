//! Tauri shell for AI Ledger.

/// Builds and runs the desktop app.
pub fn run() {
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running AI Ledger");
}

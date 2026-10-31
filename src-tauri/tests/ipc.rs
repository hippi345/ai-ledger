//! IPC tests. These live in an integration test (not `#[cfg(test)]` in the
//! lib) so the Windows Common Controls manifest from build.rs is embedded.

use ai_ledger_lib::ipc::*;

#[test]
fn app_info_reports_versions() {
    let info = app_info();
    assert_eq!(info.app_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(info.core_version, ledger_core::VERSION);
}

#[test]
fn committed_bindings_are_up_to_date() {
    let tmp = std::env::temp_dir().join(format!("ai-ledger-bindings-{}.ts", std::process::id()));
    export_bindings(&tmp).unwrap();
    let generated = std::fs::read_to_string(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);
    let committed = std::fs::read_to_string(BINDINGS_PATH).unwrap_or_default();
    let norm = |s: &str| s.replace("\r\n", "\n").trim_end().to_owned();
    assert!(
        norm(&committed) == norm(&generated),
        "ui/src/bindings.ts is stale. Regenerate it with \
         `cargo test -p ai-ledger --test ipc -- --ignored`.\n----- generated -----\n{generated}"
    );
}

/// Regenerates the committed bindings file.
#[test]
#[ignore = "writes ui/src/bindings.ts"]
fn regenerate_bindings() {
    export_bindings(std::path::Path::new(BINDINGS_PATH)).unwrap();
}

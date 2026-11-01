#![allow(clippy::unwrap_used)]

use ai_ledger_lib::commands::import::{core, ImportSpec};
use ai_ledger_lib::state::AppState;

fn fixture(name: &str) -> String {
    format!(
        "{}/../fixtures/csv_import/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn state() -> AppState {
    AppState::from_connection(ledger_core::db::open_in_memory().unwrap())
}

#[test]
fn inspect_preview_then_add_reads_the_file() {
    let i = core::inspect(&fixture("usage.csv")).unwrap();
    assert_eq!(i.format, "csv");
    assert_eq!(i.suggested.ts, "Date");
    let spec = ImportSpec {
        path: fixture("usage.csv"),
        format: i.format.clone(),
        mapping: i.suggested.clone(),
        default_app: "openai".into(),
    };
    let p = core::preview(spec.clone()).unwrap();
    assert!(p.events > 0);
    let st = state();
    let id = core::add(&st, spec).unwrap();
    let n: i64 = st
        .db()
        .query_row(
            "SELECT COUNT(*) FROM events WHERE source_id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n as u32, p.events);
}

#[test]
fn bad_specs_are_rejected_and_nothing_is_added() {
    let i = core::inspect(&fixture("usage.csv")).unwrap();
    let ok = ImportSpec {
        path: fixture("usage.csv"),
        format: "csv".into(),
        mapping: i.suggested,
        default_app: "openai".into(),
    };
    let st = state();
    let mut no_ts = ok.clone();
    no_ts.mapping.ts.clear();
    let mut no_app = ok.clone();
    no_app.default_app = " ".into();
    let mut bad_fmt = ok.clone();
    bad_fmt.format = "xml".into();
    for s in [no_ts, no_app, bad_fmt] {
        assert!(core::add(&st, s).is_err());
    }
    assert!(core::inspect("/definitely/missing.csv").is_err());
    let n: i64 = st
        .db()
        .query_row("SELECT COUNT(*) FROM sources", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

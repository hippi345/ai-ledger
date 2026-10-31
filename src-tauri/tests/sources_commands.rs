#![allow(clippy::unwrap_used)] // test helpers outside #[test] fns

use std::path::PathBuf;

use ai_ledger_lib::commands::sources::{core, Candidate};
use ai_ledger_lib::state::AppState;
use ledger_core::reader::{Cursor, RawEvent, Reader, Registry, SourceCandidate, SourceCfg, Tier};

struct Fake;

impl Reader for Fake {
    fn id(&self) -> &str {
        "fake"
    }
    fn tier(&self) -> Tier {
        Tier::Export
    }
    fn detect(&self) -> Vec<SourceCandidate> {
        vec![SourceCandidate {
            reader_id: "fake".into(),
            label: "Fake".into(),
            config: serde_json::json!({ "path": "/x" }),
        }]
    }
    fn watch_paths(&self, _: &SourceCfg) -> Vec<PathBuf> {
        vec![]
    }
    fn read_since(
        &self,
        _: &SourceCfg,
        _: Option<Cursor>,
    ) -> ledger_core::Result<(Vec<RawEvent>, Cursor)> {
        Ok((vec![], Cursor::default()))
    }
}

fn state() -> AppState {
    let mut r = Registry::new();
    r.register(Box::new(Fake)).unwrap();
    AppState::with_registry(ledger_core::db::open_in_memory().unwrap(), r)
}

#[test]
fn detect_add_list_toggle() {
    let s = state();
    let found = core::detect(&s).unwrap();
    assert_eq!(found.len(), 1);
    assert!(!found[0].added);
    let id = core::add(&s, found[0].clone()).unwrap();
    assert_eq!(core::add(&s, found[0].clone()).unwrap(), id, "deduplicated");
    assert!(core::detect(&s).unwrap()[0].added);
    let listed = core::list(&s).unwrap();
    assert_eq!(
        (listed[0].tier, listed[0].health.as_str(), listed[0].enabled),
        (2, "unknown", true)
    );
    core::set_enabled(&s, id, false).unwrap();
    assert!(!core::list(&s).unwrap()[0].enabled);
}

#[test]
fn unknown_reader_is_rejected() {
    let c = Candidate {
        reader_id: "nope".into(),
        label: "x".into(),
        config: serde_json::json!({}),
        added: false,
    };
    assert_eq!(core::add(&state(), c).unwrap_err().code, "invalid_input");
}

#[test]
fn default_registry_has_builtin_readers() {
    let r = ai_ledger_lib::state::default_registry();
    let ids: Vec<_> = r.ids().collect();
    assert_eq!(ids, ["claude_code", "csv_import", "cursor"]);
}

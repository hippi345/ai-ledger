#![allow(clippy::unwrap_used)] // test helpers outside #[test] fns
use ai_ledger_lib::commands::prices::{core, PriceOverride};
use ai_ledger_lib::state::AppState;

fn state() -> AppState {
    let mut c = ledger_core::db::open_in_memory().unwrap();
    ledger_core::prices::sync_bundled(&mut c).unwrap();
    AppState::from_connection(c)
}

fn p(input: f64) -> PriceOverride {
    PriceOverride {
        model: "my-model".into(),
        effective_from: "2026-01-01".into(),
        input,
        output: 1.0,
        cache_read: 0.0,
        cache_write: 0.0,
    }
}

#[test]
fn set_list_remove() {
    let s = state();
    assert!(core::list(&s).unwrap().is_empty());
    assert_eq!(core::set(&s, p(2.0)).unwrap(), 0);
    core::set(&s, p(3.0)).unwrap();
    assert_eq!(core::list(&s).unwrap(), vec![p(3.0)]);
    core::remove(&s, "my-model", "2026-01-01").unwrap();
    assert!(core::list(&s).unwrap().is_empty());
}

#[test]
fn bad_input_has_stable_code() {
    let e = core::set(&state(), p(-1.0)).unwrap_err();
    assert_eq!(e.code, "invalid_input");
    let e = core::remove(&state(), "nope", "2026-01-01").unwrap_err();
    assert_eq!(e.code, "invalid_input");
}

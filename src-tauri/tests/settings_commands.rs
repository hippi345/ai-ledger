#![allow(clippy::unwrap_used)] // test helpers outside #[test] fns
use ai_ledger_lib::commands::settings::core;
use ai_ledger_lib::state::AppState;
use serde_json::json;

fn state() -> AppState {
    AppState::from_connection(ledger_core::db::open_in_memory().unwrap())
}

#[test]
fn set_get_list_reset() {
    let s = state();
    assert_eq!(core::get(&s, "ui.theme").unwrap(), None);
    core::set(&s, "ui.theme", &json!("dark")).unwrap();
    core::set(&s, "retention.raw_event_days", &json!(30)).unwrap();
    assert_eq!(core::get(&s, "ui.theme").unwrap(), Some(json!("dark")));
    let keys: Vec<_> = core::list(&s).unwrap().into_iter().map(|e| e.key).collect();
    assert_eq!(keys, ["retention.raw_event_days", "ui.theme"]);
    assert!(core::reset(&s, "ui.theme").unwrap());
    assert!(!core::reset(&s, "ui.theme").unwrap());
}

#[test]
fn errors_carry_stable_codes() {
    let e = core::set(&state(), "bad key", &json!(1)).unwrap_err();
    assert_eq!(e.code, "invalid_input");
    assert!(e.message.contains("bad key"));
}

#[test]
fn launch_at_login_is_opt_in() {
    let s =
        ai_ledger_lib::state::AppState::from_connection(ledger_core::db::open_in_memory().unwrap());
    assert!(!ai_ledger_lib::autostart::wanted(&s));
    ledger_core::settings::set(&s.db(), ledger_core::settings::keys::LAUNCH_AT_LOGIN, &true)
        .unwrap();
    assert!(ai_ledger_lib::autostart::wanted(&s));
}

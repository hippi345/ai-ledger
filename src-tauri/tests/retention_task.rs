#![allow(clippy::unwrap_used)]

use ai_ledger_lib::retention::{now_utc, run_once};
use ai_ledger_lib::state::AppState;
use ledger_core::settings::{self, keys};

fn state_with_events() -> AppState {
    let s = AppState::from_connection(ledger_core::db::open_in_memory().unwrap());
    s.db()
        .execute("INSERT INTO sources (reader_id, tier) VALUES ('t', 1)", [])
        .unwrap();
    s.db()
        .execute(
            "INSERT INTO events (source_id, ts_utc, app, raw_hash) VALUES
             (1, '2026-01-01T00:00:00Z', 'cursor', 'old'),
             (1, '2026-10-31T00:00:00Z', 'cursor', 'new')",
            [],
        )
        .unwrap();
    s
}

#[test]
fn keeps_everything_by_default() {
    let s = state_with_events();
    assert_eq!(run_once(&s, "2026-11-01T00:00:00Z").unwrap(), 0);
}

#[test]
fn prunes_with_the_setting() {
    let s = state_with_events();
    settings::set(&s.db(), keys::RETENTION_DAYS, &30u32).unwrap();
    assert_eq!(run_once(&s, "2026-11-01T00:00:00Z").unwrap(), 1);
    let left: i64 = s
        .db()
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 1);
}

#[test]
fn now_is_rfc3339_utc() {
    let n = now_utc();
    assert_eq!(n.len(), 20);
    assert!(n.ends_with('Z'));
}

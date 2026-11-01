#![allow(clippy::unwrap_used)]

use ai_ledger_lib::commands::privacy::hash_existing;
use ai_ledger_lib::state::AppState;

#[test]
fn hashes_stored_project_names() {
    let s = AppState::from_connection(ledger_core::db::open_in_memory().unwrap());
    s.db()
        .execute_batch(
            "INSERT INTO sources (reader_id, tier) VALUES ('t', 1);
             INSERT INTO events (source_id, ts_utc, app, project, raw_hash)
               VALUES (1, '2026-11-01T00:00:00Z', 'cursor', 'secret', 'a');",
        )
        .unwrap();
    assert_eq!(hash_existing(&s).unwrap(), 1);
    let p: String = s
        .db()
        .query_row("SELECT project FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(p, ledger_core::privacy::hash_project("secret"));
    assert_eq!(hash_existing(&s).unwrap(), 0);
}

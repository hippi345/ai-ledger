#![allow(clippy::unwrap_used)]

use ai_ledger_lib::commands::aggregate::AggFilters;
use ai_ledger_lib::commands::sessions::{list, timeline};
use ai_ledger_lib::state::AppState;

#[test]
fn lists_sessions_and_their_timeline() {
    let s = AppState::from_connection(ledger_core::db::open_in_memory().unwrap());
    let db = s.db();
    db.execute("INSERT INTO sources (reader_id, tier) VALUES ('t', 1)", [])
        .unwrap();
    db.execute(
        "INSERT INTO sessions (id, app, started) VALUES ('s1', 'cursor', '2026-11-02T09:00:00Z')",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO events (source_id, ts_utc, app, model, session_id, input_tokens, raw_hash)
         VALUES (1, '2026-11-02T09:00:00Z', 'cursor', 'gpt-5', 's1', 7, 'a')",
        [],
    )
    .unwrap();
    drop(db);
    let l = list(&s, "2026-11-02", "2026-11-03", &AggFilters::default()).unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!((l[0].id.as_str(), l[0].events, l[0].tokens), ("s1", 1, 7.0));
    let other = AggFilters {
        app: vec!["claude_code".into()],
        ..Default::default()
    };
    assert!(list(&s, "2026-11-02", "2026-11-03", &other)
        .unwrap()
        .is_empty());
    let t = timeline(&s, "s1").unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].model.as_deref(), Some("gpt-5"));
}

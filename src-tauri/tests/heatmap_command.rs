#![allow(clippy::unwrap_used)]

use ai_ledger_lib::commands::heatmap::run;
use ai_ledger_lib::state::AppState;

#[test]
fn date_range_covers_whole_days() {
    let s = AppState::from_connection(ledger_core::db::open_in_memory().unwrap());
    s.db()
        .execute("INSERT INTO sources (reader_id, tier) VALUES ('t', 1)", [])
        .unwrap();
    for (i, ts) in [
        "2026-11-02T00:00:00Z",
        "2026-11-02T23:59:59Z",
        "2026-11-03T00:00:00Z",
    ]
    .iter()
    .enumerate()
    {
        s.db()
            .execute(
                "INSERT INTO events (source_id, ts_utc, app, input_tokens, raw_hash) VALUES (1, ?1, 'cursor', 7, ?2)",
                rusqlite::params![ts, i.to_string()],
            )
            .unwrap();
    }
    let cells = run(&s, "2026-11-02", "2026-11-03").unwrap();
    assert_eq!(cells.len(), 2);
    assert_eq!(
        (cells[0].weekday, cells[0].hour, cells[0].tokens),
        (0, 0, 7.0)
    );
    assert_eq!((cells[1].weekday, cells[1].hour), (0, 23));
}

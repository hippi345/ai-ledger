use ai_ledger_lib::state::AppState;

#[test]
fn open_creates_db_and_loads_prices() {
    let tmp = tempfile::tempdir().unwrap();
    let state = AppState::open(tmp.path()).unwrap();
    assert!(tmp.path().join("ledger.db").exists());
    let n: i64 = state
        .db()
        .query_row(
            "SELECT COUNT(*) FROM prices WHERE source = 'bundled'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(n > 0);
    drop(state);
    // Reopening is stable.
    AppState::open(tmp.path()).unwrap();
}

#[test]
fn poisoned_lock_is_recovered() {
    let state = std::sync::Arc::new(AppState::from_connection(
        ledger_core::db::open_in_memory().unwrap(),
    ));
    let s2 = state.clone();
    let _ = std::thread::spawn(move || {
        let _g = s2.db();
        panic!("poison");
    })
    .join();
    let one: i64 = state.db().query_row("SELECT 1", [], |r| r.get(0)).unwrap();
    assert_eq!(one, 1);
}

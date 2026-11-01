#![allow(clippy::unwrap_used)] // test helpers outside #[test] fns

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use ai_ledger_lib::live;
use ai_ledger_lib::state::AppState;
use ledger_core::reader::{Cursor, RawEvent, Reader, Registry, SourceCandidate, SourceCfg, Tier};
use ledger_core::sources;

/// Emits one new event per read, numbered by the cursor.
struct Counter;

impl Reader for Counter {
    fn id(&self) -> &str {
        "counter"
    }
    fn tier(&self) -> Tier {
        Tier::Native
    }
    fn detect(&self) -> Vec<SourceCandidate> {
        vec![]
    }
    fn watch_paths(&self, _: &SourceCfg) -> Vec<PathBuf> {
        vec![]
    }
    fn read_since(
        &self,
        _: &SourceCfg,
        c: Option<Cursor>,
    ) -> ledger_core::Result<(Vec<RawEvent>, Cursor)> {
        let n: u64 = c.map_or(0, |c| c.0.parse().unwrap());
        let e = RawEvent {
            ts_utc: "2026-10-01T00:00:00Z".into(),
            app: "counter".into(),
            input_tokens: 1,
            raw_hash: format!("h{n}"),
            ..Default::default()
        };
        Ok((vec![e], Cursor((n + 1).to_string())))
    }
}

fn state() -> (AppState, i64) {
    let mut r = Registry::new();
    r.register(Box::new(Counter)).unwrap();
    let s = AppState::with_registry(ledger_core::db::open_in_memory().unwrap(), r);
    let cand = SourceCandidate {
        reader_id: "counter".into(),
        label: "Counter".into(),
        config: serde_json::json!({}),
    };
    let id = sources::add(&s.db(), &cand, Tier::Native).unwrap().id;
    (s, id)
}

#[test]
fn a_burst_of_changes_syncs_once() {
    let (s, id) = state();
    let (tx, rx) = mpsc::channel();
    for _ in 0..5 {
        tx.send(id).unwrap();
    }
    drop(tx);
    let mut seen = vec![];
    live::run(&s, rx, Duration::from_millis(20), |sid, st| {
        seen.push((sid, st.inserted))
    });
    assert_eq!(seen, vec![(id, 1)]);
    assert_eq!(
        sources::cursor(&s.db(), id).unwrap(),
        Some(Cursor("1".into()))
    );
}

#[test]
fn disabled_and_unknown_sources_are_skipped() {
    let (s, id) = state();
    sources::set_enabled(&s.db(), id, false).unwrap();
    assert!(live::sync_one(&s, id).unwrap().is_none());
    assert!(live::sync_one(&s, 999).unwrap().is_none());
}

#[test]
fn poll_fallback_repeats_until_the_receiver_is_dropped() {
    let (tx, rx) = mpsc::channel();
    let h = live::spawn_poll(vec![7, 8], tx, Duration::from_millis(5)).unwrap();
    let got: Vec<i64> = (0..4)
        .map(|_| rx.recv_timeout(Duration::from_secs(5)).unwrap())
        .collect();
    assert_eq!(got, vec![7, 8, 7, 8]);
    drop(rx);
    h.join().unwrap();
}

#[test]
fn poll_fallback_is_not_started_without_ids() {
    let (tx, _rx) = mpsc::channel();
    assert!(live::spawn_poll(vec![], tx, Duration::from_millis(5)).is_none());
}

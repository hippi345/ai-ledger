//! Spec §7/§9: new usage is in the ledger within 2 s of the file write.

#![allow(clippy::unwrap_used)] // test helpers outside #[test] fns

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ai_ledger_lib::live;
use ai_ledger_lib::state::AppState;
use ledger_core::reader::{Cursor, RawEvent, Reader, Registry, SourceCandidate, SourceCfg, Tier};
use ledger_core::sources;

/// One event per line of the file in config `path`; the cursor is the line count.
struct Lines;

impl Reader for Lines {
    fn id(&self) -> &str {
        "lines"
    }
    fn tier(&self) -> Tier {
        Tier::Native
    }
    fn detect(&self) -> Vec<SourceCandidate> {
        vec![]
    }
    fn watch_paths(&self, cfg: &SourceCfg) -> Vec<PathBuf> {
        vec![PathBuf::from(cfg.config["path"].as_str().unwrap())]
    }
    fn read_since(
        &self,
        cfg: &SourceCfg,
        c: Option<Cursor>,
    ) -> ledger_core::Result<(Vec<RawEvent>, Cursor)> {
        let text =
            std::fs::read_to_string(cfg.config["path"].as_str().unwrap()).unwrap_or_default();
        let skip: usize = c.map_or(0, |c| c.0.parse().unwrap());
        let lines: Vec<&str> = text.lines().collect();
        let events = lines[skip.min(lines.len())..]
            .iter()
            .map(|l| RawEvent {
                ts_utc: "2026-10-01T00:00:00Z".into(),
                app: "lines".into(),
                input_tokens: 1,
                raw_hash: (*l).to_owned(),
                ..Default::default()
            })
            .collect();
        Ok((events, Cursor(lines.len().to_string())))
    }
}

fn count(s: &AppState) -> i64 {
    s.db()
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn a_write_is_ingested_within_two_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().canonicalize().unwrap().join("usage.log");
    let mut r = Registry::new();
    r.register(Box::new(Lines)).unwrap();
    let s = Arc::new(AppState::with_registry(
        ledger_core::db::open_in_memory().unwrap(),
        r,
    ));
    let cand = SourceCandidate {
        reader_id: "lines".into(),
        label: "Lines".into(),
        config: serde_json::json!({ "path": file }),
    };
    sources::add(&s.db(), &cand, Tier::Native).unwrap();

    let (w, rx) = live::prepare(&s).unwrap();
    assert!(w.unwatched.is_empty());
    let bg = Arc::clone(&s);
    std::thread::spawn(move || {
        live::run(&bg, rx, ledger_core::debounce::DEFAULT_WINDOW, |_, _| {});
    });
    // Let the catch-up sync and the watcher settle.
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(count(&s), 0);

    let start = Instant::now();
    std::fs::write(&file, "a\nb\n").unwrap();
    while count(&s) < 2 {
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "not ingested within 2 s"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(w);
}

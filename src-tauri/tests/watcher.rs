#![allow(clippy::unwrap_used)] // test helpers outside #[test] fns

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use ai_ledger_lib::state::AppState;
use ai_ledger_lib::watcher::{self, source_for};
use ledger_core::reader::{Cursor, RawEvent, Reader, Registry, SourceCandidate, SourceCfg, Tier};
use ledger_core::sources;

/// Watches the file named in its config's `path`.
struct FileReader;

impl Reader for FileReader {
    fn id(&self) -> &str {
        "file"
    }
    fn tier(&self) -> Tier {
        Tier::Native
    }
    fn detect(&self) -> Vec<SourceCandidate> {
        vec![]
    }
    fn watch_paths(&self, cfg: &SourceCfg) -> Vec<PathBuf> {
        cfg.config["path"]
            .as_str()
            .map(PathBuf::from)
            .into_iter()
            .collect()
    }
    fn read_since(
        &self,
        _: &SourceCfg,
        _: Option<Cursor>,
    ) -> ledger_core::Result<(Vec<RawEvent>, Cursor)> {
        Ok((vec![], Cursor::default()))
    }
}

fn state_with(paths: &[&Path]) -> (AppState, Vec<i64>) {
    let mut r = Registry::new();
    r.register(Box::new(FileReader)).unwrap();
    let s = AppState::with_registry(ledger_core::db::open_in_memory().unwrap(), r);
    let ids = paths
        .iter()
        .map(|p| {
            let cand = SourceCandidate {
                reader_id: "file".into(),
                label: "File".into(),
                config: serde_json::json!({ "path": p }),
            };
            sources::add(&s.db(), &cand, Tier::Native).unwrap().id
        })
        .collect();
    (s, ids)
}

#[test]
fn maps_changed_paths_to_sources() {
    let t = vec![(PathBuf::from("/a/db"), 1), (PathBuf::from("/b"), 2)];
    assert_eq!(source_for(&t, Path::new("/a/db")), Some(1));
    assert_eq!(source_for(&t, Path::new("/b/x/y.jsonl")), Some(2));
    assert_eq!(source_for(&t, Path::new("/a/other")), None);
}

#[test]
fn reports_a_write_to_a_file_that_did_not_exist_yet() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().canonicalize().unwrap();
    let file = dir.join("usage.db-wal");
    let (s, ids) = state_with(&[&file]);
    let (tx, rx) = mpsc::channel();
    let w = watcher::watch(&s, tx).unwrap();
    assert!(w.unwatched.is_empty());
    std::fs::write(&file, b"x").unwrap();
    assert_eq!(rx.recv_timeout(Duration::from_secs(10)).unwrap(), ids[0]);
}

#[test]
fn missing_directories_need_polling() {
    let (s, ids) = state_with(&[Path::new("/definitely/not/here/file.db")]);
    let (tx, _rx) = mpsc::channel();
    let w = watcher::watch(&s, tx).unwrap();
    assert_eq!(w.unwatched, ids);
}

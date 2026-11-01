//! File watching for live updates (spec §7): maps changes under each enabled
//! source's watch paths to that source's id.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use ledger_core::sources;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::state::AppState;

/// A running watcher plus the sources it could not watch (they need polling).
pub struct LiveWatch {
    _watcher: RecommendedWatcher,
    pub unwatched: Vec<i64>,
}

/// What to register with notify for one watch path. Directories are watched
/// recursively; a file is watched through its parent so files that do not
/// exist yet (such as a SQLite `-wal`) still report when they appear.
fn watch_root(p: &Path) -> Option<(PathBuf, RecursiveMode)> {
    if p.is_dir() {
        Some((p.to_path_buf(), RecursiveMode::Recursive))
    } else {
        p.parent()
            .filter(|d| d.is_dir())
            .map(|d| (d.to_path_buf(), RecursiveMode::NonRecursive))
    }
}

/// The source that owns a changed path, if any.
pub fn source_for(targets: &[(PathBuf, i64)], changed: &Path) -> Option<i64> {
    targets
        .iter()
        .find(|(t, _)| changed == t || changed.starts_with(t))
        .map(|(_, id)| *id)
}

/// Starts watching every enabled source and sends its id on each change.
pub fn watch(state: &AppState, tx: Sender<i64>) -> ledger_core::Result<LiveWatch> {
    let mut targets: Vec<(PathBuf, i64)> = Vec::new();
    let mut unwatched = BTreeSet::new();
    {
        let reg = state.registry();
        for s in sources::list(&state.db())?
            .into_iter()
            .filter(|s| s.enabled)
        {
            let paths = reg
                .get(&s.reader_id)
                .map(|r| r.watch_paths(&s.cfg()))
                .unwrap_or_default();
            if paths.is_empty() {
                unwatched.insert(s.id);
            }
            targets.extend(paths.into_iter().map(|p| (p, s.id)));
        }
    }
    let map = targets.clone();
    let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(ev) = res else { return };
        let ids: BTreeSet<i64> = ev
            .paths
            .iter()
            .filter_map(|p| source_for(&map, p))
            .collect();
        for id in ids {
            let _ = tx.send(id);
        }
    })
    .map_err(|e| ledger_core::Error::Reader(format!("watcher: {e}")))?;
    let mut roots = BTreeSet::new();
    for (p, id) in &targets {
        match watch_root(p) {
            Some((root, mode)) => {
                let key = (root.clone(), mode == RecursiveMode::Recursive);
                if roots.contains(&key) {
                    continue;
                }
                if let Err(e) = w.watch(&root, mode) {
                    tracing::warn!(source = id, path = %root.display(), error = %e, "watch failed");
                    unwatched.insert(*id);
                } else {
                    roots.insert(key);
                }
            }
            None => {
                unwatched.insert(*id);
            }
        }
    }
    Ok(LiveWatch {
        _watcher: w,
        unwatched: unwatched.into_iter().collect(),
    })
}

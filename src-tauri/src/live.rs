//! Live updates: debounced re-reads of changed sources (spec §7).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ledger_core::debounce::Debouncer;
use ledger_core::ingest::IngestStats;
use ledger_core::{settings, sources, sync};

use crate::state::AppState;
use crate::watcher::{self, LiveWatch};

/// Poll interval for sources that cannot be watched (spec §7).
pub const POLL_EVERY: Duration = Duration::from_secs(5);

/// Longest the loop sleeps when nothing is pending.
const IDLE: Duration = Duration::from_secs(3600);

/// True while the user has paused live updates (`live.paused`).
pub fn paused(state: &AppState) -> bool {
    settings::get_or(&state.db(), settings::keys::LIVE_PAUSED, false).unwrap_or(false)
}

/// Syncs one source by id. Missing or disabled sources are skipped (`Ok(None)`).
pub fn sync_one(state: &AppState, id: i64) -> ledger_core::Result<Option<IngestStats>> {
    let src = sources::list(&state.db())?
        .into_iter()
        .find(|s| s.id == id && s.enabled);
    let Some(src) = src else {
        return Ok(None);
    };
    let mut conn = state.db();
    let mut reg = state.registry();
    sync::sync_source(&mut conn, &mut reg, &src).map(Some)
}

/// Receives changed source ids until `changes` closes. Each id is debounced
/// by `window`, then synced, and `on_sync` gets the ingest counts. Ids still
/// pending when the channel closes are synced before returning.
pub fn run(
    state: &AppState,
    changes: Receiver<i64>,
    window: Duration,
    mut on_sync: impl FnMut(i64, IngestStats),
) {
    let mut deb = Debouncer::new(window);
    loop {
        let wait = deb.next_deadline(Instant::now()).unwrap_or(IDLE);
        let closed = match changes.recv_timeout(wait) {
            Ok(id) => {
                deb.touch(id, Instant::now());
                false
            }
            Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => true,
        };
        let now = Instant::now();
        let due = if closed {
            deb.due(now + window)
        } else {
            deb.due(now)
        };
        if !closed && paused(state) {
            // Hold changes until live updates resume; cursors stay put, so
            // nothing is lost.
            for id in due {
                deb.touch(id, now);
            }
            continue;
        }
        for id in due {
            match sync_one(state, id) {
                Ok(Some(stats)) => on_sync(id, stats),
                Ok(None) => {}
                Err(e) => tracing::warn!(source = id, error = %e, "live sync failed"),
            }
        }
        if closed {
            return;
        }
    }
}

/// Poll fallback: sends every id in `ids` each `every` until the receiver
/// is gone. Used for sources whose files cannot be watched (network
/// filesystems, permissions, missing directories).
pub fn spawn_poll(ids: Vec<i64>, tx: Sender<i64>, every: Duration) -> Option<JoinHandle<()>> {
    spawn_poll_gen(ids, tx, every, Arc::new(AtomicU64::new(0)), 0)
}

/// [`spawn_poll`] that also stops once `gen` moves past `mine` (the watcher
/// was rebuilt and a new poll thread took over).
fn spawn_poll_gen(
    ids: Vec<i64>,
    tx: Sender<i64>,
    every: Duration,
    gen: Arc<AtomicU64>,
    mine: u64,
) -> Option<JoinHandle<()>> {
    if ids.is_empty() {
        return None;
    }
    std::thread::Builder::new()
        .name("ledger-poll".into())
        .spawn(move || loop {
            std::thread::sleep(every);
            if gen.load(Ordering::SeqCst) != mine {
                return;
            }
            if ids.iter().any(|id| tx.send(*id).is_err()) {
                return;
            }
        })
        .map_err(|e| tracing::warn!(error = %e, "could not start poll fallback"))
        .ok()
}

/// Sets up live updates: starts the watcher, queues every enabled source once
/// so changes made while the app was closed are caught up, and starts the
/// poll fallback for sources that cannot be watched. Keep the returned
/// [`LiveWatch`] alive while feeding the receiver to [`run`].
pub fn prepare(state: &AppState) -> ledger_core::Result<(LiveWatch, Receiver<i64>)> {
    let (tx, rx) = mpsc::channel();
    let watch = watcher::watch(state, tx.clone())?;
    for s in sources::list(&state.db())?
        .into_iter()
        .filter(|s| s.enabled)
    {
        let _ = tx.send(s.id);
    }
    spawn_poll(watch.unwatched.clone(), tx, POLL_EVERY);
    Ok((watch, rx))
}

/// Live updates that follow source changes made while the app runs: holds
/// the change channel and the current watcher so adding, enabling or
/// disabling a source can rebuild the watcher and read the source at once.
#[derive(Default)]
pub struct Live {
    inner: Mutex<Option<LiveInner>>,
    gen: Arc<AtomicU64>,
}

struct LiveInner {
    tx: Sender<i64>,
    watch: LiveWatch,
}

impl Live {
    /// Like [`prepare`], but keeps the sender and watcher here so
    /// [`Self::reload`] can rebuild them. Returns the receiver for [`run`].
    pub fn start(&self, state: &AppState) -> ledger_core::Result<Receiver<i64>> {
        let (tx, rx) = mpsc::channel();
        self.rebuild(state, tx)?;
        for s in sources::list(&state.db())?
            .into_iter()
            .filter(|s| s.enabled)
        {
            if let Some(inner) = self.lock().as_ref() {
                let _ = inner.tx.send(s.id);
            }
        }
        Ok(rx)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<LiveInner>> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn rebuild(&self, state: &AppState, tx: Sender<i64>) -> ledger_core::Result<()> {
        let watch = watcher::watch(state, tx.clone())?;
        let mine = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        spawn_poll_gen(
            watch.unwatched.clone(),
            tx.clone(),
            POLL_EVERY,
            Arc::clone(&self.gen),
            mine,
        );
        *self.lock() = Some(LiveInner { tx, watch });
        Ok(())
    }

    /// Call after sources are added, enabled or disabled: rebuilds the
    /// watcher (and poll fallback) for the current enabled sources and, if
    /// `read` is given, queues that source for an immediate read. A no-op
    /// before [`Self::start`].
    pub fn reload(&self, state: &AppState, read: Option<i64>) {
        let Some(tx) = self.lock().as_ref().map(|i| i.tx.clone()) else {
            return;
        };
        if let Err(e) = self.rebuild(state, tx.clone()) {
            tracing::warn!(error = %e, "could not rebuild the file watcher");
        }
        if let Some(id) = read {
            let _ = tx.send(id);
        }
    }

    /// Sources currently on the poll fallback (tests and diagnostics).
    pub fn unwatched(&self) -> Vec<i64> {
        self.lock()
            .as_ref()
            .map(|i| i.watch.unwatched.clone())
            .unwrap_or_default()
    }
}

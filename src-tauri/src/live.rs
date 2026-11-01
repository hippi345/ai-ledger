//! Live updates: debounced re-reads of changed sources (spec §7).

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use ledger_core::debounce::Debouncer;
use ledger_core::ingest::IngestStats;
use ledger_core::{sources, sync};

use crate::state::AppState;

/// Longest the loop sleeps when nothing is pending.
const IDLE: Duration = Duration::from_secs(3600);

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

//! Retention pruning of raw events.
//!
//! Controlled by [`settings::keys::RETENTION_DAYS`] (0 = keep forever, the
//! default). Pruning deletes raw events (and, via cascade, their tool calls)
//! older than the cutoff. Rollups are never pruned: the cutoff is saved as
//! `retention.pruned_before`, and rollup buckets starting before it are
//! frozen so later refreshes cannot rebuild them from fewer events.

use rusqlite::Connection;

use crate::{settings, Result};

/// What a pruning pass removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PruneReport {
    /// ISO-8601 UTC cutoff; events strictly before it were removed. `None` when disabled.
    pub cutoff: Option<String>,
    pub events_deleted: usize,
}

/// Deletes events older than `days` before `now_utc` (ISO-8601). `days == 0` is a no-op.
pub fn prune_older_than(conn: &Connection, now_utc: &str, days: u32) -> Result<PruneReport> {
    if days == 0 {
        return Ok(PruneReport {
            cutoff: None,
            events_deleted: 0,
        });
    }
    let cutoff: String = conn.query_row(
        "SELECT strftime('%Y-%m-%dT%H:%M:%SZ', ?1, ?2)",
        (now_utc, format!("-{days} days")),
        |r| r.get(0),
    )?;
    let events_deleted = conn.execute("DELETE FROM events WHERE ts_utc < ?1", [&cutoff])?;
    // Freeze rollup buckets that may now cover pruned events (watermark only moves forward).
    let prev = settings::get_or(conn, settings::keys::PRUNED_BEFORE, String::new())?;
    if cutoff > prev {
        settings::set(conn, settings::keys::PRUNED_BEFORE, &cutoff)?;
    }
    tracing::info!(%cutoff, events_deleted, "retention prune");
    Ok(PruneReport {
        cutoff: Some(cutoff),
        events_deleted,
    })
}

/// Runs a pruning pass using the configured retention setting.
pub fn run(conn: &Connection, now_utc: &str) -> Result<PruneReport> {
    let days = settings::get_or(conn, settings::keys::RETENTION_DAYS, 0u32)?;
    prune_older_than(conn, now_utc, days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn seed(c: &Connection, ts: &[&str]) {
        c.execute("INSERT INTO sources (reader_id, tier) VALUES ('t', 1)", [])
            .unwrap();
        for (i, t) in ts.iter().enumerate() {
            c.execute(
                "INSERT INTO events (source_id, ts_utc, app, raw_hash) VALUES (1, ?1, 'cursor', ?2)",
                (t, i.to_string()),
            )
            .unwrap();
            c.execute(
                "INSERT INTO tool_calls (event_id, ts_utc, tool) VALUES (last_insert_rowid(), ?1, 'x')",
                [t],
            )
            .unwrap();
        }
    }

    fn count(c: &Connection, t: &str) -> i64 {
        c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
            .unwrap()
    }

    const NOW: &str = "2026-10-31T12:00:00Z";

    #[test]
    fn disabled_by_default() {
        let c = db::open_in_memory().unwrap();
        seed(&c, &["2020-01-01T00:00:00Z"]);
        let r = run(&c, NOW).unwrap();
        assert_eq!(
            r,
            PruneReport {
                cutoff: None,
                events_deleted: 0
            }
        );
        assert_eq!(count(&c, "events"), 1);
    }

    #[test]
    fn prunes_only_before_cutoff() {
        let c = db::open_in_memory().unwrap();
        seed(
            &c,
            &[
                "2026-09-30T11:59:59Z",
                "2026-10-01T12:00:00Z",
                "2026-10-31T00:00:00Z",
            ],
        );
        settings::set(&c, settings::keys::RETENTION_DAYS, &30u32).unwrap();
        let r = run(&c, NOW).unwrap();
        assert_eq!(r.cutoff.as_deref(), Some("2026-10-01T12:00:00Z"));
        assert_eq!(r.events_deleted, 1);
        assert_eq!(count(&c, "events"), 2);
        assert_eq!(
            count(&c, "tool_calls"),
            2,
            "tool calls cascade with their events"
        );
    }
}

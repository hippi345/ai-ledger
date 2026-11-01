//! Incremental sync: read one source from its saved cursor and store the result.

use rusqlite::Connection;

use crate::ingest::{upsert_events, IngestStats};
use crate::reader::Registry;
use crate::sources::{self, Source};
use crate::Result;

/// Reads `source` from its saved cursor, upserts the new events, and saves the
/// next cursor. The cursor only advances when both the read and the write
/// succeed, so a failure retries the same span next time.
pub fn sync_source(
    conn: &mut Connection,
    registry: &mut Registry,
    source: &Source,
) -> Result<IngestStats> {
    let cursor = sources::cursor(conn, source.id)?;
    let (events, next) = registry.read_source(&source.cfg(), cursor)?;
    let stats = upsert_events(conn, source.id, &events)?;
    crate::rollup::refresh_for(conn, &events)?;
    sources::save_cursor(conn, source.id, &next)?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::reader::{Cursor, RawEvent, Reader, SourceCandidate, SourceCfg, Tier};
    use crate::Error;
    use std::path::PathBuf;

    /// Emits one event per call, numbered by the cursor; fails when the cursor is "fail".
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
            cfg: &SourceCfg,
            c: Option<Cursor>,
        ) -> Result<(Vec<RawEvent>, Cursor)> {
            if cfg.config["fail"] == true {
                return Err(Error::InvalidInput("bad file".into()));
            }
            let n: u64 = c.map_or(0, |c| c.0.parse().unwrap());
            let e = RawEvent {
                ts_utc: "2026-10-01T00:00:00Z".into(),
                app: "counter".into(),
                input_tokens: n + 1,
                raw_hash: format!("h{n}"),
                ..Default::default()
            };
            Ok((vec![e], Cursor((n + 1).to_string())))
        }
    }

    fn setup(fail: bool) -> (Connection, Registry, Source) {
        let c = db::open_in_memory().unwrap();
        let s = sources::add(
            &c,
            &SourceCandidate {
                reader_id: "counter".into(),
                label: "Counter".into(),
                config: serde_json::json!({ "fail": fail }),
            },
            Tier::Native,
        )
        .unwrap();
        let mut r = Registry::new();
        r.register(Box::new(Counter)).unwrap();
        (c, r, s)
    }

    #[test]
    fn advances_cursor_and_reads_only_new_events() {
        let (mut c, mut r, s) = setup(false);
        assert_eq!(sync_source(&mut c, &mut r, &s).unwrap().inserted, 1);
        assert_eq!(sources::cursor(&c, s.id).unwrap(), Some(Cursor("1".into())));
        assert_eq!(sync_source(&mut c, &mut r, &s).unwrap().inserted, 1);
        assert_eq!(sources::cursor(&c, s.id).unwrap(), Some(Cursor("2".into())));
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
        let rolled: i64 = c
            .query_row("SELECT SUM(input_tokens) FROM rollups_day", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(rolled, 1 + 2, "rollups follow each sync");
    }

    #[test]
    fn failed_read_keeps_cursor() {
        let (mut c, mut r, s) = setup(true);
        assert!(sync_source(&mut c, &mut r, &s).is_err());
        assert_eq!(sources::cursor(&c, s.id).unwrap(), None);
    }
}

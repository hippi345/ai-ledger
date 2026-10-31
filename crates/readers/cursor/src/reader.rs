//! The Cursor [`Reader`] implementation.

use std::path::PathBuf;

use ledger_core::reader::{Cursor, RawEvent, Reader, SourceCandidate, SourceCfg, Tier};
use ledger_core::Result;
use serde::{Deserialize, Serialize};

use crate::model::{self, Composer};
use crate::paths::{CursorPaths, Env, Os};
use crate::projects;
use crate::snapshot::Snapshot;

pub const READER_ID: &str = "cursor";
const KV: &str = "cursorDiskKV";

/// Per-source config stored in `sources.config_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Cursor's root data dir (parent of `User/` and `logs/`).
    pub root: PathBuf,
}

/// Reads Cursor composer/chat usage from `globalStorage/state.vscdb`.
#[derive(Debug, Clone, Default)]
pub struct CursorReader {
    /// Overrides environment-based discovery (tests, portable installs).
    pub env: Option<Env>,
}

impl CursorReader {
    fn config(cfg: &SourceCfg) -> Result<Config> {
        Ok(serde_json::from_value(cfg.config.clone())?)
    }
}

/// Watermark cursor: the newest composer `lastUpdatedAt` (epoch ms) seen.
fn parse_cursor(c: Option<Cursor>) -> i64 {
    c.and_then(|c| c.0.parse().ok()).unwrap_or(i64::MIN)
}

impl Reader for CursorReader {
    fn id(&self) -> &str {
        READER_ID
    }

    fn tier(&self) -> Tier {
        Tier::Native
    }

    fn detect(&self) -> Vec<SourceCandidate> {
        let env = self.env.clone().unwrap_or_else(Env::from_process);
        CursorPaths::resolve(Os::current(), &env)
            .filter(CursorPaths::exists)
            .map(|p| SourceCandidate {
                reader_id: READER_ID.into(),
                label: format!("Cursor ({})", p.root.display()),
                config: serde_json::json!({ "root": p.root }),
            })
            .into_iter()
            .collect()
    }

    fn watch_paths(&self, cfg: &SourceCfg) -> Vec<PathBuf> {
        let Ok(c) = Self::config(cfg) else {
            return Vec::new();
        };
        let p = CursorPaths::new(c.root);
        let mut wal = p.global_db.clone().into_os_string();
        wal.push("-wal");
        vec![p.global_db, PathBuf::from(wal)]
    }

    fn read_since(
        &self,
        cfg: &SourceCfg,
        cursor: Option<Cursor>,
    ) -> Result<(Vec<RawEvent>, Cursor)> {
        let paths = CursorPaths::new(Self::config(cfg)?.root);
        let since = parse_cursor(cursor);
        let snap = Snapshot::take(&paths.global_db)?;
        let projects = projects::composer_projects(&paths);
        let mut events = Vec::new();
        let mut newest = since;
        for (key, value) in snap.scan_prefix(KV, model::COMPOSER_PREFIX)? {
            let Ok(composer) = serde_json::from_str::<Composer>(&value) else {
                tracing::debug!(%key, "skipping unparsable composer");
                continue;
            };
            let mark = composer.watermark();
            // `>=` re-reads the boundary composer; ingest is idempotent.
            if mark < since {
                continue;
            }
            newest = newest.max(mark);
            let id = composer.composer_id.clone();
            let project = projects.get(&id).map(String::as_str);
            events.extend(model::composer_events(&composer, project, |bubble| {
                snap.get(KV, &format!("{}{id}:{bubble}", model::BUBBLE_PREFIX))
                    .ok()
                    .flatten()
            }));
        }
        if newest == i64::MIN {
            newest = 0;
        }
        events.sort_by(|a, b| a.ts_utc.cmp(&b.ts_utc));
        Ok((events, Cursor(newest.to_string())))
    }
}

impl From<Config> for serde_json::Value {
    fn from(c: Config) -> Self {
        serde_json::json!({ "root": c.root })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Builds `<root>/User/globalStorage/state.vscdb` from fixtures/cursor/global_kv.json.
    pub(crate) fn fixture_root() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let p = CursorPaths::new(tmp.path());
        std::fs::create_dir_all(p.global_db.parent().unwrap()).unwrap();
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/cursor/global_kv.json");
        let doc: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture).unwrap()).unwrap();
        let c = rusqlite::Connection::open(&p.global_db).unwrap();
        c.execute_batch(
            "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
             CREATE TABLE cursorDiskKV (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);",
        )
        .unwrap();
        for (k, v) in doc["rows"].as_object().unwrap() {
            let text = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            c.execute(
                "INSERT INTO cursorDiskKV VALUES (?1, ?2)",
                (k, text.into_bytes()),
            )
            .unwrap();
        }
        tmp
    }

    pub(crate) fn cfg(root: &Path) -> SourceCfg {
        SourceCfg {
            source_id: 1,
            reader_id: READER_ID.into(),
            config: Config { root: root.into() }.into(),
        }
    }

    #[test]
    fn reads_assistant_bubbles_with_models_and_tokens() {
        let root = fixture_root();
        let (ev, cur) = CursorReader::default()
            .read_since(&cfg(root.path()), None)
            .unwrap();
        assert_eq!(ev.len(), 3, "one event per assistant bubble");
        assert_eq!(ev[0].ts_utc, "2026-10-31T12:00:05.000Z");
        assert_eq!(
            ev[0].model.as_deref(),
            Some("claude-4-sonnet"),
            "falls back to composer model"
        );
        assert_eq!((ev[0].input_tokens, ev[0].output_tokens), (1200, 340));
        assert_eq!(ev[1].model.as_deref(), Some("gpt-5"), "bubble model wins");
        assert_eq!(ev[1].session_id.as_deref(), Some("c-1"));
        assert_eq!(
            ev[2].ts_utc, "2026-11-01T12:00:00.000Z",
            "falls back to composer createdAt"
        );
        assert!(ev[2].model.is_none());
        assert_eq!(cur.0, "1793534460000");
        assert!(ev.iter().all(|e| e.app == "cursor" && e.requests == 1));
    }

    #[test]
    fn cursor_skips_older_composers_and_hashes_are_stable() {
        let root = fixture_root();
        let r = CursorReader::default();
        let (all, cur) = r.read_since(&cfg(root.path()), None).unwrap();
        let (again, cur2) = r.read_since(&cfg(root.path()), Some(cur.clone())).unwrap();
        assert_eq!(again.len(), 1, "only the boundary composer is re-read");
        assert_eq!(cur2, cur);
        assert_eq!(again[0].raw_hash, all[2].raw_hash);
    }

    #[test]
    fn ingest_roundtrip_is_idempotent() {
        let root = fixture_root();
        let mut db = ledger_core::db::open_in_memory().unwrap();
        db.execute(
            "INSERT INTO sources (reader_id, tier) VALUES ('cursor', 1)",
            [],
        )
        .unwrap();
        let (ev, _) = CursorReader::default()
            .read_since(&cfg(root.path()), None)
            .unwrap();
        ledger_core::ingest::upsert_events(&mut db, 1, &ev).unwrap();
        let s = ledger_core::ingest::upsert_events(&mut db, 1, &ev).unwrap();
        assert_eq!(s.unchanged, 3);
    }

    #[test]
    fn tool_calls_and_projects() {
        let root = fixture_root();
        crate::projects::tests::write_workspaces(root.path());
        let (ev, _) = CursorReader::default()
            .read_since(&cfg(root.path()), None)
            .unwrap();
        assert_eq!(ev[0].project.as_deref(), Some("ai ledger"));
        assert_eq!(ev[2].project.as_deref(), Some("randobot"));
        let t0 = &ev[0].tool_calls[0];
        assert_eq!(
            (t0.tool.as_str(), t0.mcp_server.as_deref(), t0.ok),
            ("read_file", None, true)
        );
        let t1 = &ev[1].tool_calls[0];
        assert_eq!(
            (t1.tool.as_str(), t1.mcp_server.as_deref(), t1.ok),
            ("create_issue", Some("github"), false)
        );
        assert_eq!(t1.duration_ms, Some(420));
        assert!(ev[2].tool_calls.is_empty());
    }

    #[test]
    fn detect_and_watch_paths() {
        let root = fixture_root();
        let r = CursorReader {
            env: Some(Env {
                xdg_config_home: Some(root.path().join("nope")),
                ..Env::default()
            }),
        };
        assert!(r.detect().is_empty() || cfg!(any(target_os = "macos", windows)));
        let w = r.watch_paths(&cfg(root.path()));
        assert!(w[0].ends_with("globalStorage/state.vscdb"));
        assert!(w[1].to_string_lossy().ends_with("state.vscdb-wal"));
    }
}

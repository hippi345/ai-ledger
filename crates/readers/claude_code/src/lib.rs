//! Tier 1 reader for Claude Code's local transcripts.
//!
//! Claude Code writes one JSONL file per session under
//! `~/.claude/projects/<encoded cwd>/<sessionId>.jsonl` (`CLAUDE_CONFIG_DIR`
//! overrides `~/.claude`). Assistant lines carry `message.model`,
//! `message.usage` (input, output, cache read/creation tokens), and
//! `tool_use` content blocks. A streamed reply is written as several lines
//! sharing one `message.id`; usage on the last line is the final total, so
//! those lines share a `raw_hash` and later lines update the stored event.
//!
//! Only metadata is read: message text and tool inputs are never deserialized.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use ledger_core::ingest::{raw_hash, NewEvent, NewToolCall};
use ledger_core::reader::{Cursor, RawEvent, Reader, SourceCandidate, SourceCfg, Tier};
use ledger_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub const READER_ID: &str = "claude_code";
pub const APP: &str = "claude_code";

/// Per-source config: the Claude config dir (normally `~/.claude`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub root: PathBuf,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Line {
    #[serde(rename = "type")]
    kind: String,
    uuid: Option<String>,
    request_id: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
    timestamp: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
    content: Content,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    cache_read_input_tokens: u64,
    cache_creation_input_tokens: u64,
    /// Newer transcripts split cache writes by lifetime.
    cache_creation: Option<CacheCreation>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CacheCreation {
    ephemeral_5m_input_tokens: u64,
    ephemeral_1h_input_tokens: u64,
}

impl Usage {
    /// `(5-minute, 1-hour)` cache writes. Without the split, all writes
    /// count as standard 5-minute writes.
    fn cache_writes(&self) -> (u64, u64) {
        match &self.cache_creation {
            Some(c) => (c.ephemeral_5m_input_tokens, c.ephemeral_1h_input_tokens),
            None => (self.cache_creation_input_tokens, 0),
        }
    }
}

/// `content` is either a string (ignored) or a list of typed blocks.
#[derive(Default)]
struct Content(Vec<String>);

impl<'de> Deserialize<'de> for Content {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Block {
            #[serde(rename = "type")]
            kind: String,
            name: Option<String>,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Blocks(Vec<Block>),
            Other(serde::de::IgnoredAny),
        }
        Ok(Content(match Raw::deserialize(d)? {
            Raw::Blocks(b) => b
                .into_iter()
                .filter(|b| b.kind == "tool_use")
                .filter_map(|b| b.name)
                .collect(),
            Raw::Other(_) => Vec::new(),
        }))
    }
}

/// Splits `mcp__<server>__<tool>` into `(server, tool)`.
pub fn split_mcp(name: &str) -> Option<(&str, &str)> {
    name.strip_prefix("mcp__")?
        .split_once("__")
        .filter(|(s, t)| !s.is_empty() && !t.is_empty())
}

fn project_from_cwd(cwd: &str) -> Option<String> {
    let name = cwd
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()?;
    (!name.is_empty()).then(|| name.to_owned())
}

/// Converts one JSONL line into an event (assistant lines with usage only).
fn line_event(raw: &str, file_session: &str) -> Option<NewEvent> {
    let l: Line = serde_json::from_str(raw).ok()?;
    if l.kind != "assistant" {
        return None;
    }
    let msg = l.message?;
    let usage = msg.usage?;
    let model = msg.model.filter(|m| !m.is_empty() && !m.starts_with('<'));
    // `<synthetic>` placeholder replies (e.g. errors) carry no real usage.
    model.as_ref()?;
    let ts = l.timestamp?;
    let session = l.session_id.unwrap_or_else(|| file_session.to_owned());
    let key = msg
        .id
        .map(|id| format!("msg:{id}:{}", l.request_id.unwrap_or_default()))
        .or_else(|| l.uuid.map(|u| format!("uuid:{u}")))?;
    let (cache_5m, cache_1h) = usage.cache_writes();
    let tool_calls = msg
        .content
        .0
        .iter()
        .map(|name| {
            let (server, tool) =
                split_mcp(name).map_or((None, name.as_str()), |(s, t)| (Some(s), t));
            NewToolCall {
                ts_utc: ts.clone(),
                tool: tool.to_owned(),
                mcp_server: server.map(str::to_owned),
                duration_ms: None,
                ok: true,
            }
        })
        .collect();
    Some(NewEvent {
        raw_hash: raw_hash([APP, &session, &key]),
        ts_utc: ts,
        app: APP.into(),
        model,
        project: l.cwd.as_deref().and_then(project_from_cwd),
        session_id: Some(session),
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_read_tokens: usage.cache_read_input_tokens,
        cache_write_tokens: cache_5m,
        cache_write_1h_tokens: cache_1h,
        requests: 1,
        tool_calls,
        ..Default::default()
    })
}

/// Reads complete lines of `path` from byte `offset`. Returns events and the
/// offset just past the last complete line (a partially written trailing
/// line is left for the next read).
pub fn read_file_from(path: &Path, offset: u64) -> Result<(Vec<NewEvent>, u64)> {
    let mut f = std::fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let len = f.metadata().map_err(|e| Error::io(path, e))?.len();
    // Truncated or replaced file: start over (ingest is idempotent).
    let start = if offset > len { 0 } else { offset };
    f.seek(SeekFrom::Start(start))
        .map_err(|e| Error::io(path, e))?;
    let session = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut r = BufReader::new(f);
    let mut pos = start;
    let mut events = Vec::new();
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = r.read_line(&mut buf).map_err(|e| Error::io(path, e))?;
        if n == 0 || !buf.ends_with('\n') {
            break;
        }
        pos += n as u64;
        if let Some(e) = line_event(buf.trim_end(), &session) {
            events.push(e);
        }
    }
    Ok((events, pos))
}

/// All `*.jsonl` transcripts under `<root>/projects`, sorted.
pub fn transcripts(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(dirs) = std::fs::read_dir(root.join("projects")) else {
        return out;
    };
    for d in dirs.filter_map(|e| e.ok()) {
        let Ok(files) = std::fs::read_dir(d.path()) else {
            continue;
        };
        out.extend(
            files
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "jsonl")),
        );
    }
    out.sort();
    out
}

/// Default Claude config dir: `$CLAUDE_CONFIG_DIR` or `~/.claude`.
pub fn default_root() -> Option<PathBuf> {
    let var = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    var("CLAUDE_CONFIG_DIR").or_else(|| {
        var("HOME")
            .or_else(|| var("USERPROFILE"))
            .map(|h| h.join(".claude"))
    })
}

/// Claude Code [`Reader`]. The cursor is a JSON map of file path → byte offset.
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeReader {
    /// Overrides [`default_root`] for detection (tests).
    pub root: Option<PathBuf>,
}

impl Reader for ClaudeCodeReader {
    fn id(&self) -> &str {
        READER_ID
    }

    fn tier(&self) -> Tier {
        Tier::Native
    }

    fn detect(&self) -> Vec<SourceCandidate> {
        self.root
            .clone()
            .or_else(default_root)
            .filter(|r| r.join("projects").is_dir())
            .map(|r| SourceCandidate {
                reader_id: READER_ID.into(),
                label: format!("Claude Code ({})", r.display()),
                config: serde_json::json!({ "root": r }),
            })
            .into_iter()
            .collect()
    }

    fn watch_paths(&self, cfg: &SourceCfg) -> Vec<PathBuf> {
        serde_json::from_value::<Config>(cfg.config.clone())
            .map(|c| vec![c.root.join("projects")])
            .unwrap_or_default()
    }

    fn read_since(
        &self,
        cfg: &SourceCfg,
        cursor: Option<Cursor>,
    ) -> Result<(Vec<RawEvent>, Cursor)> {
        let root = serde_json::from_value::<Config>(cfg.config.clone())?.root;
        let mut offsets: BTreeMap<String, u64> = cursor
            .and_then(|c| serde_json::from_str(&c.0).ok())
            .unwrap_or_default();
        let mut events = Vec::new();
        for file in transcripts(&root) {
            let key = file.to_string_lossy().into_owned();
            let from = offsets.get(&key).copied().unwrap_or(0);
            match read_file_from(&file, from) {
                Ok((ev, pos)) => {
                    events.extend(ev);
                    offsets.insert(key, pos);
                }
                Err(e) => tracing::warn!(file = %key, error = %e, "skipping transcript"),
            }
        }
        Ok((events, Cursor(serde_json::to_string(&offsets)?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fixture_root() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let src =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/claude_code/projects");
        for d in std::fs::read_dir(src).unwrap() {
            let d = d.unwrap().path();
            let dest = tmp.path().join("projects").join(d.file_name().unwrap());
            std::fs::create_dir_all(&dest).unwrap();
            for f in std::fs::read_dir(&d).unwrap() {
                let f = f.unwrap().path();
                std::fs::copy(&f, dest.join(f.file_name().unwrap())).unwrap();
            }
        }
        tmp
    }

    fn cfg(root: &Path) -> SourceCfg {
        SourceCfg {
            source_id: 1,
            reader_id: READER_ID.into(),
            config: serde_json::json!({ "root": root }),
        }
    }

    #[test]
    fn parses_usage_models_tools_and_projects() {
        let root = fixture_root();
        let (ev, _) = ClaudeCodeReader::default()
            .read_since(&cfg(root.path()), None)
            .unwrap();
        assert_eq!(
            ev.len(),
            3,
            "user, summary, junk and synthetic lines are skipped"
        );
        assert_eq!(
            ev[0].raw_hash, ev[1].raw_hash,
            "streamed lines share one message"
        );
        assert_eq!(ev[1].output_tokens, 95);
        assert_eq!(
            (ev[1].cache_read_tokens, ev[1].cache_write_tokens),
            (12000, 500)
        );
        assert_eq!(ev[1].tool_calls[0].tool, "Read");
        assert_eq!(ev[2].model.as_deref(), Some("claude-opus-4-1"));
        let t = &ev[2].tool_calls[0];
        assert_eq!(
            (t.mcp_server.as_deref(), t.tool.as_str()),
            (Some("github"), "create_issue")
        );
        assert_eq!(ev[2].project.as_deref(), Some("ai-ledger"));
        assert_eq!(ev[2].session_id.as_deref(), Some("s-1"));
    }

    #[test]
    fn ingest_collapses_streamed_lines() {
        let root = fixture_root();
        let (ev, _) = ClaudeCodeReader::default()
            .read_since(&cfg(root.path()), None)
            .unwrap();
        let mut db = ledger_core::db::open_in_memory().unwrap();
        db.execute(
            "INSERT INTO sources (reader_id, tier) VALUES ('claude_code', 1)",
            [],
        )
        .unwrap();
        ledger_core::ingest::upsert_events(&mut db, 1, &ev).unwrap();
        let (n, out): (i64, i64) = db
            .query_row("SELECT COUNT(*), SUM(output_tokens) FROM events", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((n, out), (2, 135), "final streamed usage wins");
    }

    #[test]
    fn incremental_reads_resume_at_complete_lines() {
        let root = fixture_root();
        let r = ClaudeCodeReader::default();
        let (_, cur) = r.read_since(&cfg(root.path()), None).unwrap();
        let (none, cur) = r.read_since(&cfg(root.path()), Some(cur)).unwrap();
        assert!(none.is_empty());
        let file = transcripts(root.path()).remove(0);
        let line = r#"{"type":"assistant","uuid":"a9","sessionId":"s-1","timestamp":"2026-10-31T13:00:00Z","message":{"id":"msg_9","model":"claude-sonnet-4-5","usage":{"input_tokens":1,"output_tokens":2}}}"#;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap();
        write!(f, "{}", &line[..40]).unwrap();
        let (partial, cur) = r.read_since(&cfg(root.path()), Some(cur)).unwrap();
        assert!(partial.is_empty(), "partial line is not consumed");
        writeln!(f, "{}", &line[40..]).unwrap();
        let (ev, _) = r.read_since(&cfg(root.path()), Some(cur)).unwrap();
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].output_tokens, 2);
    }

    #[test]
    fn splits_5m_and_1h_cache_writes() {
        let line = r#"{"type":"assistant","uuid":"a","sessionId":"s","timestamp":"2026-10-31T12:00:00Z","message":{"id":"m","model":"claude-sonnet-4-5","usage":{"input_tokens":1,"output_tokens":1,"cache_creation_input_tokens":300,"cache_creation":{"ephemeral_5m_input_tokens":100,"ephemeral_1h_input_tokens":200}}}}"#;
        let e = line_event(line, "s").unwrap();
        assert_eq!((e.cache_write_tokens, e.cache_write_1h_tokens), (100, 200));
        let old = line.replace(r#","cache_creation":{"ephemeral_5m_input_tokens":100,"ephemeral_1h_input_tokens":200}"#, "");
        let e = line_event(&old, "s").unwrap();
        assert_eq!((e.cache_write_tokens, e.cache_write_1h_tokens), (300, 0));
    }

    #[test]
    fn detect_requires_projects_dir() {
        let root = fixture_root();
        assert_eq!(
            ClaudeCodeReader {
                root: Some(root.path().into())
            }
            .detect()
            .len(),
            1
        );
        assert!(ClaudeCodeReader {
            root: Some(root.path().join("x"))
        }
        .detect()
        .is_empty());
    }
}

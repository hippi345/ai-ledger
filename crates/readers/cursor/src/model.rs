//! Parsing Cursor's composer (chat/agent) records into usage events.
//!
//! Layout in `globalStorage/state.vscdb`, table `cursorDiskKV`:
//! - `composerData:<composerId>`: one conversation, with `createdAt` /
//!   `lastUpdatedAt` (epoch ms), an optional `modelConfig.modelName`, and
//!   `fullConversationHeadersOnly` listing bubbles in order.
//! - `bubbleId:<composerId>:<bubbleId>`: one message. `type` 1 = user,
//!   2 = assistant. Assistant bubbles may carry `tokenCount` and `modelInfo`.
//!
//! Each assistant bubble becomes one event (`requests = 1`). Message text is
//! never read into the ledger. Fields are parsed leniently because Cursor's
//! storage format is undocumented and changes between versions.

use serde::Deserialize;
use serde_json::Value;

use ledger_core::ingest::{raw_hash, NewEvent, NewToolCall};

pub const APP: &str = "cursor";
pub const COMPOSER_PREFIX: &str = "composerData:";
pub const BUBBLE_PREFIX: &str = "bubbleId:";
const ASSISTANT: i64 = 2;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Composer {
    pub composer_id: String,
    pub created_at: Option<i64>,
    pub last_updated_at: Option<i64>,
    pub model_config: Option<ModelName>,
    pub full_conversation_headers_only: Vec<Header>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelName {
    pub model_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Header {
    pub bubble_id: String,
    #[serde(rename = "type")]
    pub kind: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TokenCount {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// The parts of a bubble we use. Text fields are deliberately not declared.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Bubble {
    #[serde(rename = "type")]
    pub kind: i64,
    pub created_at: Option<Value>,
    pub model_info: Option<ModelName>,
    pub token_count: Option<TokenCount>,
    pub tool_former_data: Option<ToolData>,
    /// Only the *length* of the message text is kept, for token estimation.
    pub text: TextLen,
    pub timing_info: Option<Timing>,
}

/// Character count of a text field. The text itself is dropped during
/// deserialization and never stored.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TextLen(pub usize);

impl<'de> Deserialize<'de> for TextLen {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s: Option<String> = Option::deserialize(d)?;
        Ok(TextLen(s.map_or(0, |s| s.chars().count())))
    }
}

/// Rough chars-to-tokens heuristic (~4 characters per token, rounded up).
/// Used only when Cursor did not record token counts; such events are
/// flagged `estimated`.
pub const CHARS_PER_TOKEN: usize = 4;

pub fn estimate_tokens(chars: usize) -> u64 {
    u64::try_from(chars.div_ceil(CHARS_PER_TOKEN)).unwrap_or(u64::MAX)
}

/// Tool invocation metadata on an assistant bubble. Arguments and results are
/// deliberately not declared, so they are never read.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ToolData {
    pub name: Option<String>,
    pub status: Option<String>,
    pub mcp_server_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Timing {
    pub client_start_time: Option<i64>,
    pub client_end_time: Option<i64>,
}

/// Splits Cursor's `mcp_<server>_<tool>` naming into `(server, tool)`.
/// Server names containing `_` are ambiguous; an explicit `mcpServerName` wins.
pub fn split_mcp(name: &str) -> Option<(&str, &str)> {
    name.strip_prefix("mcp_")?
        .split_once('_')
        .filter(|(s, t)| !s.is_empty() && !t.is_empty())
}

fn tool_call(data: ToolData, ts: &str, timing: Option<Timing>) -> Option<NewToolCall> {
    let name = data.name.filter(|n| !n.is_empty())?;
    let (server, tool) = match (
        data.mcp_server_name.filter(|s| !s.is_empty()),
        split_mcp(&name),
    ) {
        (Some(s), Some((_, t))) => (Some(s), t.to_owned()),
        (Some(s), None) => (Some(s), name.clone()),
        (None, Some((s, t))) => (Some(s.to_owned()), t.to_owned()),
        (None, None) => (None, name.clone()),
    };
    let duration_ms = timing
        .and_then(|t| Some(t.client_end_time? - t.client_start_time?))
        .and_then(|d| u64::try_from(d).ok());
    let ok = !matches!(
        data.status.as_deref(),
        Some("error" | "failed" | "cancelled")
    );
    Some(NewToolCall {
        ts_utc: ts.to_owned(),
        tool,
        mcp_server: server,
        duration_ms,
        ok,
    })
}

impl Composer {
    /// Last activity (epoch ms), used as the incremental-read watermark.
    pub fn watermark(&self) -> i64 {
        self.last_updated_at.or(self.created_at).unwrap_or(0)
    }

    pub fn model(&self) -> Option<&str> {
        self.model_config
            .as_ref()?
            .model_name
            .as_deref()
            .filter(|m| !m.is_empty())
    }
}

/// Formats epoch milliseconds as `YYYY-MM-DDTHH:MM:SS.mmmZ`.
pub fn iso_from_ms(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (days, sod) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        sod / 3600,
        sod % 3600 / 60,
        sod % 60,
        ms.rem_euclid(1000)
    )
}

/// Normalizes a bubble timestamp (ISO string or epoch ms) to ISO-8601 UTC.
fn bubble_ts(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) if s.len() >= 19 => Some(s.clone()),
        Value::Number(n) => n.as_i64().map(iso_from_ms),
        _ => None,
    }
}

/// Builds events for one composer. `lookup(bubble_id)` returns the raw bubble JSON.
/// `project` is the workspace the composer belongs to, when known.
pub fn composer_events(
    c: &Composer,
    project: Option<&str>,
    mut lookup: impl FnMut(&str) -> Option<String>,
) -> Vec<NewEvent> {
    let fallback_ts = iso_from_ms(c.created_at.or(c.last_updated_at).unwrap_or(0));
    let mut out = Vec::new();
    // Characters of conversation the model has seen so far (for estimation).
    let mut context_chars = 0usize;
    for h in &c.full_conversation_headers_only {
        let bubble: Bubble = lookup(&h.bubble_id)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let own_chars = bubble.text.0;
        if h.kind != ASSISTANT {
            context_chars += own_chars;
            continue;
        }
        let reported = bubble
            .token_count
            .filter(|t| t.input_tokens > 0 || t.output_tokens > 0);
        let estimated = reported.is_none();
        let (input_tokens, output_tokens) = match reported {
            Some(t) => (t.input_tokens, t.output_tokens),
            None => (estimate_tokens(context_chars), estimate_tokens(own_chars)),
        };
        context_chars += own_chars;
        let model = bubble
            .model_info
            .and_then(|m| m.model_name)
            .filter(|m| !m.is_empty())
            .or_else(|| c.model().map(str::to_owned));
        let ts = bubble_ts(bubble.created_at.as_ref()).unwrap_or_else(|| fallback_ts.clone());
        let tool_calls = bubble
            .tool_former_data
            .and_then(|d| tool_call(d, &ts, bubble.timing_info))
            .into_iter()
            .collect();
        out.push(NewEvent {
            ts_utc: ts,
            project: project.map(str::to_owned),
            tool_calls,
            app: APP.into(),
            model,
            session_id: Some(c.composer_id.clone()),
            input_tokens,
            output_tokens,
            requests: 1,
            estimated,
            raw_hash: raw_hash([APP, &c.composer_id, &h.bubble_id]),
            ..Default::default()
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_conversion() {
        assert_eq!(iso_from_ms(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_from_ms(1_793_448_000_123), "2026-10-31T12:00:00.123Z");
        assert_eq!(iso_from_ms(951_782_400_000), "2000-02-29T00:00:00.000Z");
    }

    #[test]
    fn mcp_names() {
        assert_eq!(
            split_mcp("mcp_github_create_issue"),
            Some(("github", "create_issue"))
        );
        assert_eq!(split_mcp("read_file"), None);
        assert_eq!(split_mcp("mcp__x"), None);
        let t = tool_call(
            ToolData {
                name: Some("mcp_a_b".into()),
                status: None,
                mcp_server_name: Some("my_srv".into()),
            },
            "t",
            None,
        )
        .unwrap();
        assert_eq!(
            (t.mcp_server.as_deref(), t.tool.as_str(), t.ok),
            (Some("my_srv"), "b", true)
        );
    }

    #[test]
    fn estimates_when_tokens_missing() {
        let c: Composer = serde_json::from_str(
            r#"{"composerId":"e","createdAt":0,"fullConversationHeadersOnly":[
                {"bubbleId":"u","type":1},{"bubbleId":"a","type":2},
                {"bubbleId":"u2","type":1},{"bubbleId":"a2","type":2}]}"#,
        )
        .unwrap();
        let bubbles = std::collections::HashMap::from([
            ("u", r#"{"type":1,"text":"0123456789"}"#),
            (
                "a",
                r#"{"type":2,"text":"abcdef","tokenCount":{"inputTokens":0,"outputTokens":0}}"#,
            ),
            ("u2", r#"{"type":1,"text":"xy"}"#),
            (
                "a2",
                r#"{"type":2,"tokenCount":{"inputTokens":7,"outputTokens":1}}"#,
            ),
        ]);
        let ev = composer_events(&c, None, |id| bubbles.get(id).map(|s| s.to_string()));
        assert_eq!(ev.len(), 2);
        assert!(ev[0].estimated);
        assert_eq!(
            (ev[0].input_tokens, ev[0].output_tokens),
            (3, 2),
            "10 chars ctx, 6 chars out"
        );
        assert!(!ev[1].estimated, "reported counts are used as-is");
        assert_eq!((ev[1].input_tokens, ev[1].output_tokens), (7, 1));
        assert_eq!(estimate_tokens(0), 0);
    }

    #[test]
    fn lenient_parsing() {
        let c: Composer = serde_json::from_str(r#"{"composerId":"x","unknown":1}"#).unwrap();
        assert_eq!(c.watermark(), 0);
        assert!(c.model().is_none());
        let b: Bubble = serde_json::from_str(r#"{"type":2,"text":"secret"}"#).unwrap();
        assert!(b.token_count.is_none());
    }
}

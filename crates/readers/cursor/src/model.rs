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

use ledger_core::ingest::{raw_hash, NewEvent};

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
pub fn composer_events(
    c: &Composer,
    mut lookup: impl FnMut(&str) -> Option<String>,
) -> Vec<NewEvent> {
    let fallback_ts = iso_from_ms(c.created_at.or(c.last_updated_at).unwrap_or(0));
    let mut out = Vec::new();
    for h in c
        .full_conversation_headers_only
        .iter()
        .filter(|h| h.kind == ASSISTANT)
    {
        let bubble: Bubble = lookup(&h.bubble_id)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let tokens = bubble.token_count.unwrap_or_default();
        let model = bubble
            .model_info
            .and_then(|m| m.model_name)
            .filter(|m| !m.is_empty())
            .or_else(|| c.model().map(str::to_owned));
        out.push(NewEvent {
            ts_utc: bubble_ts(bubble.created_at.as_ref()).unwrap_or_else(|| fallback_ts.clone()),
            app: APP.into(),
            model,
            session_id: Some(c.composer_id.clone()),
            input_tokens: tokens.input_tokens,
            output_tokens: tokens.output_tokens,
            requests: 1,
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
    fn lenient_parsing() {
        let c: Composer = serde_json::from_str(r#"{"composerId":"x","unknown":1}"#).unwrap();
        assert_eq!(c.watermark(), 0);
        assert!(c.model().is_none());
        let b: Bubble = serde_json::from_str(r#"{"type":2,"text":"secret"}"#).unwrap();
        assert!(b.token_count.is_none());
    }
}

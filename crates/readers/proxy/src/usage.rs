//! Usage extraction from proxied responses (no network code here).

use ledger_core::cost::ServiceTier;
use ledger_core::ingest::{raw_hash, NewEvent};
use serde_json::Value;

/// Upstream API family, chosen by the proxy route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Anthropic,
    OpenAi,
}

impl Vendor {
    /// The `app` name recorded on events.
    pub fn app(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
        }
    }
}

fn u(v: &Value, path: &[&str]) -> u64 {
    path.iter()
        .try_fold(v, |v, k| v.get(k))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn s(v: &Value, path: &[&str]) -> Option<String> {
    path.iter()
        .try_fold(v, |v, k| v.get(k))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Fields gathered from one response (possibly across stream events).
#[derive(Debug, Default)]
struct Acc {
    id: Option<String>,
    model: Option<String>,
    tier: Option<String>,
    usage: Option<Value>,
    /// Anthropic streams send input usage in `message_start` and the
    /// cumulative output count in `message_delta`.
    output_override: Option<u64>,
}

impl Acc {
    fn absorb(&mut self, vendor: Vendor, obj: &Value) {
        // Responses API streams wrap the final object in `response`;
        // Anthropic streams wrap it in `message`.
        let inner = match (vendor, s(obj, &["type"]).as_deref()) {
            (Vendor::Anthropic, Some("message_start")) => obj.get("message"),
            (Vendor::Anthropic, Some("message_delta")) => {
                if let Some(n) = obj.get("usage").and_then(|u| u.get("output_tokens")) {
                    self.output_override = n.as_u64();
                }
                return;
            }
            (Vendor::OpenAi, Some(t)) if t.starts_with("response.") => obj.get("response"),
            _ => Some(obj),
        };
        let Some(o) = inner else { return };
        if let Some(id) = s(o, &["id"]) {
            self.id.get_or_insert(id);
        }
        if let Some(m) = s(o, &["model"]) {
            self.model = Some(m);
        }
        if let Some(t) = s(o, &["service_tier"]).or_else(|| s(o, &["usage", "service_tier"])) {
            self.tier = Some(t);
        }
        if let Some(us) = o.get("usage").filter(|v| v.is_object()) {
            self.usage = Some(us.clone());
        }
    }

    fn finish(self, vendor: Vendor, ts_utc: &str) -> Option<NewEvent> {
        let us = self.usage?;
        let mut ev = NewEvent {
            ts_utc: ts_utc.to_owned(),
            app: vendor.app().to_owned(),
            model: self.model,
            requests: 1,
            ..NewEvent::default()
        };
        match vendor {
            Vendor::Anthropic => {
                ev.input_tokens = u(&us, &["input_tokens"]);
                ev.output_tokens = self
                    .output_override
                    .unwrap_or_else(|| u(&us, &["output_tokens"]));
                ev.cache_read_tokens = u(&us, &["cache_read_input_tokens"]);
                if us.get("cache_creation").is_some_and(Value::is_object) {
                    ev.cache_write_tokens =
                        u(&us, &["cache_creation", "ephemeral_5m_input_tokens"]);
                    ev.cache_write_1h_tokens =
                        u(&us, &["cache_creation", "ephemeral_1h_input_tokens"]);
                } else {
                    ev.cache_write_tokens = u(&us, &["cache_creation_input_tokens"]);
                }
                // Only standard and batch are recorded; priority bills as
                // standard (same rule as the Claude Code reader).
                ev.service_tier = match self.tier.as_deref() {
                    Some("standard") => Some(ServiceTier::Standard),
                    Some("batch") => Some(ServiceTier::Batch),
                    _ => None,
                };
            }
            Vendor::OpenAi => {
                // Chat Completions: prompt/completion_tokens; Responses API:
                // input/output_tokens. Cached tokens are included in the
                // prompt count, so they are split out here.
                let chat = us.get("prompt_tokens").is_some();
                let (inp, out, cached) = if chat {
                    (
                        u(&us, &["prompt_tokens"]),
                        u(&us, &["completion_tokens"]),
                        u(&us, &["prompt_tokens_details", "cached_tokens"]),
                    )
                } else {
                    (
                        u(&us, &["input_tokens"]),
                        u(&us, &["output_tokens"]),
                        u(&us, &["input_tokens_details", "cached_tokens"]),
                    )
                };
                ev.input_tokens = inp.saturating_sub(cached);
                ev.cache_read_tokens = cached;
                ev.output_tokens = out;
                ev.service_tier = self.tier.as_deref().and_then(ServiceTier::parse);
            }
        }
        ev.raw_hash = match &self.id {
            Some(id) => raw_hash(["proxy", vendor.app(), id.as_str()]),
            None => raw_hash(["proxy", vendor.app(), ts_utc, &us.to_string()]),
        };
        Some(ev)
    }
}

/// Builds a usage event from a complete response body, or `None` when the
/// body carries no usage. `ts_utc` is when the proxy saw the request.
pub fn event_from_response(vendor: Vendor, body: &[u8], ts_utc: &str) -> Option<NewEvent> {
    let text = std::str::from_utf8(body).ok()?;
    let mut acc = Acc::default();
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        acc.absorb(vendor, &v);
    } else {
        for line in text.lines() {
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            if let Ok(v) = serde_json::from_str::<Value>(data.trim()) {
                acc.absorb(vendor, &v);
            }
        }
    }
    acc.finish(vendor, ts_utc)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TS: &str = "2026-11-01T12:00:00Z";

    #[test]
    fn anthropic_json() {
        let body = br#"{"id":"msg_1","type":"message","model":"claude-sonnet-4-5","content":[{"type":"text","text":"SECRET"}],"usage":{"input_tokens":10,"output_tokens":20,"cache_read_input_tokens":30,"cache_creation_input_tokens":300,"cache_creation":{"ephemeral_5m_input_tokens":100,"ephemeral_1h_input_tokens":200},"service_tier":"standard"}}"#;
        let ev = event_from_response(Vendor::Anthropic, body, TS).unwrap();
        assert_eq!(ev.app, "anthropic");
        assert_eq!(ev.model.as_deref(), Some("claude-sonnet-4-5"));
        assert_eq!(
            (ev.input_tokens, ev.output_tokens, ev.cache_read_tokens),
            (10, 20, 30)
        );
        assert_eq!(
            (ev.cache_write_tokens, ev.cache_write_1h_tokens),
            (100, 200)
        );
        assert_eq!(ev.service_tier, Some(ServiceTier::Standard));
        assert_eq!(ev.requests, 1);
        assert!(!ev.estimated);
        assert!(!format!("{ev:?}").contains("SECRET"));
    }

    #[test]
    fn anthropic_priority_is_not_recorded_and_old_cache_field_is_5m() {
        let body = br#"{"id":"msg_2","model":"m","usage":{"input_tokens":1,"output_tokens":1,"cache_creation_input_tokens":7,"service_tier":"priority"}}"#;
        let ev = event_from_response(Vendor::Anthropic, body, TS).unwrap();
        assert_eq!(ev.service_tier, None);
        assert_eq!((ev.cache_write_tokens, ev.cache_write_1h_tokens), (7, 0));
    }

    #[test]
    fn anthropic_stream_uses_final_output_count() {
        let body = b"event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_3\",\"model\":\"claude-haiku-4-5\",\"usage\":{\"input_tokens\":5,\"output_tokens\":1}}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"SECRET\"}}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":42}}\n\n";
        let ev = event_from_response(Vendor::Anthropic, body, TS).unwrap();
        assert_eq!(ev.model.as_deref(), Some("claude-haiku-4-5"));
        assert_eq!((ev.input_tokens, ev.output_tokens), (5, 42));
        assert!(!format!("{ev:?}").contains("SECRET"));
    }

    #[test]
    fn openai_chat_splits_cached_tokens() {
        let body = br#"{"id":"chatcmpl-1","model":"gpt-5","service_tier":"flex","choices":[{"message":{"content":"SECRET"}}],"usage":{"prompt_tokens":100,"completion_tokens":7,"prompt_tokens_details":{"cached_tokens":60}}}"#;
        let ev = event_from_response(Vendor::OpenAi, body, TS).unwrap();
        assert_eq!(
            (ev.input_tokens, ev.cache_read_tokens, ev.output_tokens),
            (40, 60, 7)
        );
        assert_eq!(ev.service_tier, Some(ServiceTier::Flex));
        assert!(!format!("{ev:?}").contains("SECRET"));
    }

    #[test]
    fn openai_responses_stream_completed_event() {
        let body = b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"SECRET\"}\n\n\
data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"model\":\"gpt-5-mini\",\"service_tier\":\"default\",\"usage\":{\"input_tokens\":50,\"output_tokens\":9,\"input_tokens_details\":{\"cached_tokens\":10}}}}\n\n";
        let ev = event_from_response(Vendor::OpenAi, body, TS).unwrap();
        assert_eq!(ev.model.as_deref(), Some("gpt-5-mini"));
        assert_eq!(
            (ev.input_tokens, ev.cache_read_tokens, ev.output_tokens),
            (40, 10, 9)
        );
        assert_eq!(ev.service_tier, Some(ServiceTier::Standard));
    }

    #[test]
    fn stream_without_usage_yields_nothing() {
        let body = b"data: {\"id\":\"chatcmpl-2\",\"model\":\"gpt-5\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n\n";
        assert!(event_from_response(Vendor::OpenAi, body, TS).is_none());
        assert!(event_from_response(Vendor::OpenAi, b"not json", TS).is_none());
        assert!(event_from_response(Vendor::OpenAi, &[0xff, 0xfe], TS).is_none());
    }

    #[test]
    fn hash_is_stable_per_response_id() {
        let a = br#"{"id":"msg_9","model":"m","usage":{"input_tokens":1,"output_tokens":1}}"#;
        let h1 = event_from_response(Vendor::Anthropic, a, TS)
            .unwrap()
            .raw_hash;
        let h2 = event_from_response(Vendor::Anthropic, a, "2026-11-02T00:00:00Z")
            .unwrap()
            .raw_hash;
        assert_eq!(h1, h2);
        let o = event_from_response(Vendor::OpenAi, a, TS).unwrap().raw_hash;
        assert_ne!(h1, o);
    }
}

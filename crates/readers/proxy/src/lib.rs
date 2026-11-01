//! Tier 3 reader: an opt-in logging proxy on localhost (spec §5, item 21).
//!
//! This module turns a proxied API response into a usage event. It keeps
//! **metadata only**: model, token counts, service tier and the response id.
//! Prompt and completion text is never stored. Request bodies aren't read
//! at all, and response text is dropped as soon as the usage fields are
//! pulled out.
//!
//! It understands JSON and server-sent-event (streaming) responses from the
//! Anthropic Messages API and from the OpenAI Chat Completions and
//! Responses APIs. If a response has no usage block (for example an OpenAI
//! stream without `stream_options.include_usage`), no event is produced;
//! token counts are never guessed.

pub mod server;
pub mod usage;

use std::path::PathBuf;

use ledger_core::reader::{Cursor, RawEvent, Reader, SourceCandidate, SourceCfg, Tier};

pub const READER_ID: &str = "proxy";

/// Default localhost port for the proxy.
pub const DEFAULT_PORT: u16 = 8788;

/// Registry entry for the proxy source. The proxy pushes events as it sees
/// them, so there is nothing to detect, watch or re-read; this only keeps
/// the source healthy and labeled.
#[derive(Debug, Default)]
pub struct ProxyReader;

impl Reader for ProxyReader {
    fn id(&self) -> &str {
        READER_ID
    }
    fn tier(&self) -> Tier {
        Tier::Capture
    }
    fn detect(&self) -> Vec<SourceCandidate> {
        Vec::new()
    }
    fn watch_paths(&self, _cfg: &SourceCfg) -> Vec<PathBuf> {
        Vec::new()
    }
    fn read_since(
        &self,
        _cfg: &SourceCfg,
        cursor: Option<Cursor>,
    ) -> ledger_core::Result<(Vec<RawEvent>, Cursor)> {
        Ok((Vec::new(), cursor.unwrap_or_default()))
    }
}

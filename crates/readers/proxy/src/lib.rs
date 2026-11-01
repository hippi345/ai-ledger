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

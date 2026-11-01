# Writing a reader

A reader turns one app's local data into usage events. Readers live in
`crates/readers/<name>/` and implement `ledger_core::reader::Reader`:

```rust
pub trait Reader: Send + Sync {
    fn id(&self) -> &str;              // stable, e.g. "cursor"; stored in the DB
    fn tier(&self) -> Tier;            // Native | Export | Capture
    fn detect(&self) -> Vec<SourceCandidate>;
    fn watch_paths(&self, cfg: &SourceCfg) -> Vec<PathBuf>;
    fn read_since(&self, cfg: &SourceCfg, cursor: Option<Cursor>)
        -> Result<(Vec<RawEvent>, Cursor)>;
}
```

The `csv_import` reader is the smallest complete example to copy.

## Rules

1. **Read only.** Never write to, lock, or migrate the app's files. Open
   SQLite databases read-only.
2. **No network.** Readers must not open sockets or depend on HTTP clients.
   `src-tauri/tests/no_network.rs` fails CI if they do.
3. **`detect` is cheap and quiet.** Check known paths and return an empty
   list on any problem. Don't panic or log errors for a missing app.
4. **Errors are isolated.** Returning `Err`, or even panicking, from
   `read_since` only marks that source unhealthy (`Registry::read_source`
   catches both). Prefer a clear `Error::Reader("...")` message, because the
   user sees it.
5. **Resume with the cursor.** The cursor is an opaque string you own, such
   as a file offset, the last row id, or a file stamp. Return events after
   it and the next cursor. The same input must give the same output.
6. **Never double count.** Set `raw_hash` with `ledger_core::ingest::raw_hash`
   over fields that identify the record (for example the source path plus
   the line or row id). Ingest upserts on it, so re-reading is safe.
7. **Don't guess.** Leave unknown fields as `None` or 0. If a number is
   derived rather than read, such as a token split inferred from a total,
   set `estimated: true`. Set `service_tier` only from a recorded value.
8. **Timestamps are UTC ISO-8601** (`ts_utc`).

## Event fields (`NewEvent`)

`ts_utc`, `app`, `model`, `project`, `session_id`; `input_tokens`,
`output_tokens`, `cache_read_tokens`, `cache_write_tokens` (5-minute),
`cache_write_1h_tokens`, `cache_write_12h_tokens`, `requests`;
`service_tier`, `estimated`, `raw_hash`, `tool_calls`.

Cost is not set by the reader. It is computed from the price table at
query time.

## Tests and fixtures

- Put small, **anonymized** sample inputs in `fixtures/<reader>/`. Don't
  commit real prompts, code, paths, or keys.
- Add golden tests that parse the fixtures and assert the exact events.
  Also test that a second read with the returned cursor yields nothing new,
  and that corrupt input gives `Err` rather than a panic or wrong numbers.

## Registering

1. Add the crate to the workspace `Cargo.toml` and as a dependency of
   `src-tauri`.
2. Register it in `src-tauri/src/state.rs`:
   `r.register(Box::new(my_reader::MyReader::default()))`.
3. Add any new models to the prices table (with a source URL and date)
   so its costs aren't reported as unknown.

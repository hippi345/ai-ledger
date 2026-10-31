# AI Ledger

A local-first desktop app that records, aggregates, and visualizes AI usage
(tokens, estimated cost, requests, tool/MCP calls) across the AI apps on one
machine. Cursor is supported first.

Nothing leaves your machine: no telemetry, no cloud, no LLM calls.

## Status

Early development. See the [design spec](docs/superpowers/specs/2026-10-31-ai-ledger-design.md)
for the architecture and PR plan.

## Layout

```
src-tauri/            Tauri v2 shell (app, commands, tray, watcher)
crates/ledger-core/   storage, rollups, prices
crates/readers/       pluggable usage readers (cursor, claude_code, csv_import, proxy)
ui/                   React + TypeScript frontend
fixtures/             golden reader fixtures
docs/                 design and guides
```

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## License

MIT © 2026 Joel Shearon

### Running the app

Requires Node 20+ and the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
npm --prefix ui ci
ui/node_modules/.bin/tauri dev   # from the repo root
```

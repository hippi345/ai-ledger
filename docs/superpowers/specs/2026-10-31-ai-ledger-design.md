# AI Ledger — Design Spec

Date: 2026-10-31 · Status: Draft · License: MIT

## 1. Goal
AI Ledger is a local-first desktop app that records, aggregates, and visualizes AI usage (tokens, cost, requests, tool/MCP calls) across AI apps on one machine. Cursor is supported first. Nothing leaves the machine: no telemetry, no cloud, no LLM calls.

## 2. Non-goals
- No multi-user or cloud sync in v1.
- No billing reconciliation with vendor invoices. Cost is an estimate from the local price table.
- No interception of network traffic by default (that is opt-in only in Tier 3).

## 3. Platforms and stack
- **Shell:** Tauri v2 on macOS (arm64/x64), Windows 10+ (x64), and Linux (x64: AppImage, deb, rpm).
- **Backend:** Rust (stable), tokio, rusqlite (bundled SQLite, WAL), notify (file watching), serde, thiserror, and tracing.
- **Frontend:** React 18, TypeScript, Vite, TanStack Query/Router, Zustand, ECharts (charts plus PNG export), react-grid-layout, and Tailwind.
- **IPC:** typed Tauri commands and events. Types are generated with specta / tauri-specta.

## 4. Storage (fully local, persistent SQLite)
- The database is ledger.db in the OS app-data dir (Tauri path API). It uses WAL mode and versioned migrations (refinery-style, embedded).
- **Tables:**
  - `sources` (id, reader_id, tier, config_json, enabled, last_cursor)
  - `events` (id, source_id, ts_utc, app, model, project, session_id, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, requests, cost_usd_est, price_version, raw_hash UNIQUE)
  - `tool_calls` (id, event_id, ts_utc, tool, mcp_server NULL, duration_ms, ok)
  - `sessions` (id, app, project, started, ended)
  - `prices` (model, effective_from, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok, source)
  - `rollups_{day,week,month}`
  - `settings`, `saved_views`, `layouts`
- **Idempotency:** ingest uses raw_hash with upsert, so re-reading a file never double counts.
- **Retention:** configurable. Raw events can be pruned while rollups are kept.
- **Backup:** export/import of the .db file, plus a VACUUM INTO snapshot.

## 5. Pluggable readers (3 tiers)
Every reader implements this Rust trait:
```rust
trait Reader { fn id(&self)->&str; fn tier(&self)->Tier; fn detect(&self)->Vec<SourceCandidate>;
  fn watch_paths(&self, cfg:&SourceCfg)->Vec<PathBuf>;
  fn read_since(&self, cfg:&SourceCfg, cursor:Option<Cursor>)->Result<(Vec<RawEvent>,Cursor)>; }
```
Readers are registered in a static registry. Each runs in an isolated task, and a panic or error marks only that source unhealthy.

- **Tier 1, Native local data:** reads the app's own on-disk logs or DBs. This tier gives the highest fidelity and needs zero config.
  - **Cursor (first):** state.vscdb / workspace storage SQLite (read-only, copy-on-read to avoid lock contention) plus its local logs. It extracts model, tokens where present, tool and MCP calls, and project from the workspace path. When tokens are absent it estimates them, and those events are flagged `estimated=1`.
  - **Follow-ons:** Claude Code (~/.claude JSONL), Codex CLI, Continue, and Aider.
- **Tier 2, Exported/structured files:** user-pointed CSV/JSON usage exports (e.g. vendor dashboards) with column mapping.
- **Tier 3, Opt-in capture:** a local OpenAI/Anthropic-compatible logging proxy on localhost that records usage metadata only (no prompt bodies), plus manual entry. It is off by default.

**Fixtures:** each reader ships golden fixtures per OS path layout.

## 6. Price table
- A bundled prices.json is versioned and dated, and the user can edit it in Settings. Overrides are stored in `prices` with source=user.
- Cost = Σ tokens × rate effective at the event ts. Each event stores price_version, and a "recompute costs" action re-prices history.
- Unknown models show "—" cost and are listed in Settings for pricing.
- Updates arrive with app releases only (no network fetch).

## 7. Live updates (file watching)
- notify watchers sit on every enabled reader's watch_paths, debounced at 250 ms. On change, the backend runs the reader's read_since(cursor), upserts the results, incrementally updates affected rollup buckets, and emits `ledger://ingested` {range, dims}.
- **Target:** new usage is visible in the UI within 1–2 s of the file write (p95 ≤ 2 s, enforced by a test).
- If a watch fails (network FS, permissions), a poll fallback runs every 5 s.
- **Tray mode:** closing the window minimizes to the system tray and ingestion continues. The tray menu shows today's tokens/cost and offers Open, Pause live, and Quit. Launch-at-login is optional (tauri-plugin-autostart).

## A. Aggregation layer (local, deterministic)
- Rollups: daily, weekly, and monthly buckets, broken down by app, model, tool, MCP server, and project, plus session. Materialized tables are refreshed incrementally on ingest.
- Metrics: input, output, and cached tokens; estimated cost (local price table); request count; tool-call count; active minutes.
- Top-N rankings for each dimension and period (N configurable, rest grouped as "Other").
- Period-over-period deltas (absolute and %) against the previous equal-length period.
- One query API, `aggregate(range, groupBy[], filters{}, metric)`, that every page and chart uses.

## B. Plain-English summary cards (templated, no LLM call)
- Example: "This week: 4.2M tokens, ~$18, mostly Cursor + claude-sonnet, up 30%."
- Cards for today, this week, and this month, plus cards for biggest mover, top tool, and top MCP.
- Built from aggregate output with number formatting (K/M, ~$) and delta wording rules (up/down/flat within ±5%).

## C. Charts
- Stacked area of tokens or cost over time, by app or model.
- Bar and donut breakdowns by app, model, project, or MCP.
- Heatmap of activity by hour of day × day of week.
- Bar chart of tool calls (top-N tools).
- Cost trend line with a dashed projection to the end of the period.

## D. Full dashboard experience
- Sidebar nav: Overview, Apps, Models, Tools & MCP, Projects, Sessions, Costs, Settings.
- Overview is a customizable widget grid: add, remove, rearrange, and resize KPI tiles, summary cards, and charts. The layout is saved locally and can be reset to the default.
- A global date-range picker (presets plus custom) and a filter bar (app/model/tool/MCP/project chips) apply on every page and are synced to the URL.
- Cross-filtering: clicking a chart segment adds that value to the global filters. The filter chip shows its source, and Esc or the chip's × removes it.
- Saved views: a name plus range, filters, and layout, stored locally. They can be switched from the sidebar.
- Export: CSV for any chart or table's underlying data, PNG for any widget or the whole page.
- Live-updating tiles: they refresh on ingest events (file watcher or local websocket), with a "live" indicator and a pause toggle.
- Detail pages: each dimension page shows its KPIs, a trend chart, and a sortable table. Sessions has a list view and a drill-down timeline. Costs shows the price table, a projection, and budget alerts. Settings covers sources, prices, privacy, and retention.
## 8. Privacy and security
- No outbound network: the CSP blocks remote origins and the updater is disabled unless opted in.
- Prompt/response content is never stored. Paths are stored, and there is an option to hash project names.
- Source DBs are opened read-only.

## 9. Testing
- **Rust:** unit tests per reader against fixtures, property tests (proptest) for rollups (rollup = Σ events), migration tests, and price-calculation tests.
- **Ingest latency:** an integration test writes to a fixture file and asserts the event is in the DB plus the event is emitted within 2 s.
- **Frontend:** Vitest plus Testing Library for components and summary text, and Playwright (tauri-driver) E2E for nav, filters, cross-filter, layout persistence, and export.
- **CI:** GitHub Actions matrix (macOS, Windows, Ubuntu) running fmt, clippy -D warnings, tests, and tauri build artifacts.

## 10. Repo layout
```
src-tauri/ (app, commands, tray, watcher)  crates/ledger-core (db, rollups, prices)
crates/readers/{cursor,claude_code,csv_import,proxy}  ui/ (React)  fixtures/  docs/
```

## 11. PR plan (each PR small and independently reviewable)
**Foundation**
1. Repo scaffold: MIT LICENSE, README, cargo workspace, rustfmt/clippy config
2. Tauri v2 + React/Vite/TS app skeleton
3. CI matrix (3 OSes): fmt, clippy, test
4. tauri-specta typed IPC plumbing
5. Logging/tracing plus error types

**Storage**
6. SQLite open (WAL), app-data path, migration runner
7. Schema v1: sources, events, tool_calls, sessions
8. Idempotent upsert via raw_hash
9. Settings table plus commands
10. Backup export/import and VACUUM INTO
11. Retention pruning job

**Readers**
12. Reader trait, registry, per-source isolation and health
13. Source auto-detect plus Sources settings UI
14. Cursor reader: path discovery for 3 OSes
15. Cursor reader: read-only copy-on-read DB access
16. Cursor reader: events and models plus fixtures
17. Cursor reader: tool/MCP calls plus project mapping
18. Cursor reader: token estimation fallback flag
19. Claude Code JSONL reader
20. Tier 2 CSV/JSON import with column mapping
21. Tier 3 local logging proxy (opt-in, metadata only)
22. Manual entry

**Prices**
23. Bundled prices.json plus loader
24. Cost calculation at ingest plus price_version
25. User price overrides plus recompute
26. Unknown-models list

**Live updates**
27. notify watcher, debounce, and incremental read_since
28. Poll fallback
29. Ingest event bus to frontend (`ledger://ingested`)
30. Latency integration test (≤ 2 s)
31. Tray mode: minimize to tray plus today's stats menu
32. Pause live plus launch-at-login

**Aggregation**
33. Rollup schema plus incremental rollup job
34. `aggregate()` query API plus property tests
35. Top-N with "Other"
36. Period-over-period deltas

**Summaries**
37. Summary text generator plus tests
38. Summary cards UI

**Charts**
39. Chart primitives and theme (ECharts)
40. Stacked area over time
41. Bar/donut breakdowns
42. Hour×day heatmap
43. Tool-call bar chart
44. Cost trend line plus projection

**Dashboard shell**
45. Sidebar nav plus routes
46. Global date-range picker (URL-synced)
47. Global filter bar
48. Cross-filtering from chart clicks
49. Widget grid: add/remove
50. Widget grid: rearrange/resize
51. Layout persistence plus reset
52. Saved views
53. CSV export
54. PNG export (widget and page)
55. Live-updating tiles plus live indicator

**Pages**
56. Overview default layout
57. Apps page
58. Models page
59. Tools & MCP page
60. Projects page
61. Sessions list plus drill-down timeline
62. Costs page (prices, projection, budgets)
63. Settings page (sources, prices, privacy, retention)

**Release**
64. Privacy hardening: CSP, no-network audit test
65. Playwright E2E suite
66. Packaging and signing config for 3 OSes
67. Docs: user guide plus reader-authoring guide

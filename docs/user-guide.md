# AI Ledger user guide

AI Ledger reads usage logs that AI apps already keep on your machine. It
shows tokens, estimated cost, requests, and tool or MCP calls. It makes no
network calls, sends no telemetry, and has no account.

## First run

1. Install a build (see [packaging](packaging.md); builds are unsigned for
   now).
2. Open **Settings → Sources** and click **Detect sources**. AI Ledger
   lists what it found (Cursor, Claude Code). Click **Add** on the ones you
   want. Each one is read right away and then watched for changes.
3. Exported usage files (CSV or JSON, for example from a vendor dashboard)
   are read by the *CSV/JSON import* reader. It maps columns onto ledger
   fields; JSON fields use dot paths such as `usage.input`. A row that
   can't be parsed is skipped and counted, never guessed, and importing the
   same file again does not double count. There is no in-app screen for
   setting up the mapping yet.

## Pages

| Page | Shows |
|---|---|
| Overview | Totals, trend over time, top apps and models, activity heatmap |
| Apps / Models / Projects | Sortable breakdowns for that dimension; click a row to filter by it |
| Tools & MCP | Tool and MCP server call counts and failures |
| Sessions | Sessions in the range and a per-session timeline (lists stop at 500 sessions; narrow the range to see more) |
| Costs | Spend by period, projections, and budgets |
| Settings | Sources, prices, privacy, retention, startup |

The **date range** and any **filters** (shown as chips at the top) stay set
as you move between pages. They are kept in the URL. Days are bucketed in
**UTC**. Tables can be exported as CSV, and charts as PNG, through a save
dialog.

## Costs are estimates

Costs come from the built-in price table (Settings → Prices), which records
a source and date for each price. You can override any price. Events with
an unknown model are listed under **Unknown models** so you can price them.
A value marked *estimated* was derived rather than read directly (for
example, when a source records no token split). The vendor service tier is
recorded when a source provides it; Anthropic's priority tier is priced as
standard.

**Budgets** have three levels: *near* (80% of the budget used), *projected
over* (on pace to exceed it), and *over*.

## Privacy

- **Hash project names:** new events store a hash instead of the project
  name. **Hash existing project names** rewrites the data already stored,
  including rollups. This cannot be undone, so you are asked to confirm.
- **Retention:** event-level data older than the chosen period is pruned.
  Daily rollups are kept.
- Data lives in a local SQLite file in the app data directory. Uninstalling
  the app does not delete it; remove that directory to erase everything.

## Local logging proxy (optional, off by default)

Settings → **Local logging proxy** runs a small proxy on `127.0.0.1` for
API calls that no reader can see, such as your own scripts. Set your
client's base URL to `http://127.0.0.1:8788/anthropic` (Anthropic) or
`http://127.0.0.1:8788/openai/v1` (OpenAI).
The proxy forwards only to `api.anthropic.com` and `api.openai.com`. It
rejects requests from browsers and from any host other than localhost.
It records only the model and token counts. Prompts, responses and API
keys are never stored or logged. This is the only part of AI Ledger that
makes network requests, and it does so only while it is on.

## Startup and tray

Under **Settings → Startup** you can launch AI Ledger at login. The tray
menu shows today's total and has **Pause live updates**.

## Troubleshooting

- **A source shows as unhealthy:** its error is shown in Settings →
  Sources. Other sources keep working. The app may have changed its log
  format; please open an issue with the error text and **no** log contents.
- **No data:** check that the source is enabled and that the date range
  covers when you used the app.

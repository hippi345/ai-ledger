import { useEffect, useState } from "react";
import { commands, type TimelineItem_Serialize } from "../bindings";
import { formatCost, formatTokens } from "../charts/theme";
import { ExportCsvButton } from "../export/ExportCsvButton";

const HEADER = [
  "ts_utc",
  "kind",
  "model",
  "tokens",
  "cost_usd",
  "tool",
  "mcp_server",
  "duration_ms",
  "ok",
];
const toRow = (t: TimelineItem_Serialize) => [
  t.tsUtc,
  t.tool ? "tool" : "request",
  t.model,
  t.tokens,
  t.costUsd,
  t.tool,
  t.mcpServer,
  t.durationMs,
  t.ok,
];
/** `2026-11-01T10:05:07Z` -> `10:05:07`. */
const time = (ts: string) => ts.slice(11, 19);

type Props = { id: string; onBack: () => void };

/** One session's requests and tool calls in time order (UTC). */
export function SessionTimeline({ id, onBack }: Props) {
  const [items, setItems] = useState<TimelineItem_Serialize[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    setItems(null);
    void commands.sessionTimeline(id).then((r) => {
      if (!live) return;
      if (r.status === "ok") setItems(r.data);
      else setError(r.error.message);
    });
    return () => {
      live = false;
    };
  }, [id]);
  return (
    <section aria-label={`Session ${id}`}>
      <button type="button" onClick={onBack}>
        ← All sessions
      </button>
      <h3>Session {id}</h3>
      {error && <p role="alert">{error}</p>}
      {!error && !items && (
        <p className="skeleton" aria-busy="true">
          <span className="sr-only">Loading…</span>
        </p>
      )}
      {items && items.length === 0 && (
        <p className="empty">
          No events left for this session (retention may have pruned them).
        </p>
      )}
      {items && items.length > 0 && (
        <>
          <p>{items[0].tsUtc.slice(0, 10)} UTC</p>
          <ExportCsvButton
            name={`Session ${id}`}
            header={HEADER}
            rows={items.map(toRow)}
          />
          <ol aria-label="Timeline">
            {items.map((t, i) => (
              <li
                key={i}
                className={t.tool ? (t.ok ? "tool" : "tool failed") : "request"}
              >
                <time dateTime={t.tsUtc}>{time(t.tsUtc)}</time>{" "}
                {t.tool ? (
                  <>
                    Tool {t.tool}
                    {t.mcpServer && ` (${t.mcpServer})`}
                    {t.durationMs != null && ` · ${t.durationMs} ms`}
                    {!t.ok && " · failed"}
                  </>
                ) : (
                  <>
                    {t.model ?? "Unknown model"} · {formatTokens(t.tokens ?? 0)}{" "}
                    tokens
                    {t.costUsd != null && ` · ${formatCost(t.costUsd)}`}
                  </>
                )}
              </li>
            ))}
          </ol>
        </>
      )}
    </section>
  );
}

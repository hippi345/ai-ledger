import { formatCost, formatTokens } from "../charts/theme";
import { ExportCsvButton } from "../export/ExportCsvButton";
import type { SessionRow_Serialize } from "../bindings";
import { useSessions } from "./useSessions";

const HEADER = [
  "session",
  "app",
  "project",
  "first",
  "last",
  "events",
  "tokens",
  "cost_usd",
  "models",
];
const toRow = (s: SessionRow_Serialize) => [
  s.id,
  s.app,
  s.project,
  s.firstTs,
  s.lastTs,
  s.events,
  s.tokens,
  s.costUsd,
  s.models,
];
/** `2026-11-01T10:05:00Z` -> `2026-11-01 10:05` (UTC). */
const when = (ts: string) => ts.slice(0, 16).replace("T", " ");

/** Matches ledger_core::sessions::LIST_LIMIT. */
export const SESSION_LIMIT = 500;

type Props = { onOpen?: (id: string) => void };

/** Sessions list, newest first; selecting one opens its timeline. */
export function SessionsPage({ onOpen }: Props) {
  const { sessions, error } = useSessions();
  if (error) return <p role="alert">{error}</p>;
  if (!sessions) return <p>Loading…</p>;
  if (sessions.length === 0) return <p>No sessions in this range.</p>;
  return (
    <>
      {sessions.length >= SESSION_LIMIT && (
        <p role="note">
          Showing the first {SESSION_LIMIT} sessions (newest first). Narrow the
          range or filters to see the rest.
        </p>
      )}
      <ExportCsvButton
        name="Sessions"
        header={HEADER}
        rows={sessions.map(toRow)}
      />
      <table>
        <caption>Sessions (UTC)</caption>
        <thead>
          <tr>
            <th scope="col">Last activity</th>
            <th scope="col">App</th>
            <th scope="col">Project</th>
            <th scope="col">Models</th>
            <th scope="col">Events</th>
            <th scope="col">Tokens</th>
            <th scope="col">Cost</th>
          </tr>
        </thead>
        <tbody>
          {sessions.map((s) => (
            <tr key={s.id}>
              <td>
                {onOpen ? (
                  <button
                    type="button"
                    aria-label={`Open session ${s.id}`}
                    onClick={() => onOpen(s.id)}
                  >
                    {when(s.lastTs)}
                  </button>
                ) : (
                  when(s.lastTs)
                )}
              </td>
              <td>{s.app}</td>
              <td>{s.project ?? "Unknown"}</td>
              <td>{s.models || "—"}</td>
              <td>{s.events}</td>
              <td>{formatTokens(s.tokens ?? 0)}</td>
              <td>{s.costUsd == null ? "—" : formatCost(s.costUsd)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}

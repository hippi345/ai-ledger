import type { Candidate, SourceInfo_Serialize as SourceInfo } from "../bindings";

const READER_NAMES: Record<string, string> = {
  cursor: "Cursor",
  claude_code: "Claude Code",
  csv_import: "CSV/JSON import",
  manual: "Manual entries",
};

const TIER_NAMES: Record<number, string> = {
  1: "Native data",
  2: "Exported file",
  3: "Opt-in capture",
};

export function readerName(id: string): string {
  return READER_NAMES[id] ?? id;
}

export interface SourcesPanelProps {
  sources: SourceInfo[];
  candidates: Candidate[];
  busy?: boolean;
  onDetect: () => void;
  onAdd: (candidate: Candidate) => void;
  onToggle: (id: number, enabled: boolean) => void;
}

/** Settings → Sources: configured sources with health, plus detected ones to add. */
export function SourcesPanel({ sources, candidates, busy, onDetect, onAdd, onToggle }: SourcesPanelProps) {
  const pending = candidates.filter((c) => !c.added);
  return (
    <section aria-labelledby="sources-heading">
      <h2 id="sources-heading">Sources</h2>
      {sources.length === 0 ? (
        <p>No sources yet. Detect the AI apps on this machine to start tracking usage.</p>
      ) : (
        <ul aria-label="Configured sources">
          {sources.map((s) => (
            <li key={s.id}>
              <label>
                <input
                  type="checkbox"
                  checked={s.enabled}
                  disabled={busy}
                  onChange={(e) => onToggle(s.id, e.currentTarget.checked)}
                />
                {readerName(s.readerId)}
              </label>{" "}
              <span>{TIER_NAMES[s.tier] ?? `Tier ${s.tier}`}</span>{" "}
              <span role="status" data-health={s.health}>
                {s.health === "unhealthy" ? `Error: ${s.error ?? "unknown"}` : s.health}
              </span>
            </li>
          ))}
        </ul>
      )}
      <button type="button" onClick={onDetect} disabled={busy}>
        Detect sources
      </button>
      {pending.length > 0 && (
        <ul aria-label="Detected sources">
          {pending.map((c) => (
            <li key={`${c.readerId}:${JSON.stringify(c.config)}`}>
              {c.label}{" "}
              <button type="button" onClick={() => onAdd(c)} disabled={busy}>
                Add
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

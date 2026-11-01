import { useEffect, useState } from "react";
import { commands } from "../bindings";

/** ledger_core::settings::keys::RETENTION_DAYS (0 or unset = keep forever). */
export const RETENTION_KEY = "retention.raw_event_days";
export const RETENTION_CHOICES = [0, 30, 90, 180, 365] as const;

const label = (d: number) => (d === 0 ? "Keep forever" : `${d} days`);

/** Picks how long raw events are kept; daily totals (rollups) are always kept. */
export function RetentionPanel() {
  const [days, setDays] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void commands.getSetting(RETENTION_KEY).then((r) => {
      if (r.status === "ok") setDays(typeof r.data === "number" ? r.data : 0);
      else setError(r.error.message);
    });
  }, []);
  const choose = async (d: number) => {
    const r =
      d === 0
        ? await commands.resetSetting(RETENTION_KEY)
        : await commands.setSetting(RETENTION_KEY, d);
    if (r.status === "ok") {
      setDays(d);
      setError(null);
    } else setError(r.error.message);
  };
  return (
    <section aria-labelledby="retention-h">
      <h3 id="retention-h">Retention</h3>
      {error && <p role="alert">{error}</p>}
      <label>
        Keep raw events{" "}
        <select
          value={days ?? 0}
          disabled={days === null}
          onChange={(e) => void choose(Number(e.target.value))}
        >
          {RETENTION_CHOICES.map((d) => (
            <option key={d} value={d}>
              {label(d)}
            </option>
          ))}
        </select>
      </label>
      <p>
        Older raw events (sessions, timelines, heatmap) are deleted daily.
        Daily, weekly and monthly totals are kept forever.
      </p>
    </section>
  );
}

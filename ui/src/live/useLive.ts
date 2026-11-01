import { useCallback, useEffect, useState } from "react";
import { commands } from "../bindings";
import { useIngested } from "./useIngested";

/** Setting the backend's live loop checks (ledger_core::settings::keys::LIVE_PAUSED). */
export const LIVE_PAUSED_KEY = "live.paused";

/**
 * Live state for the indicator: whether live updates are paused, and when
 * usage was last ingested. The pause flag is re-read when the window gains
 * focus, since the tray menu can toggle it too.
 */
export function useLive() {
  const [paused, setPaused] = useState(false);
  const [lastIngest, setLastIngest] = useState<Date | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    const r = await commands.getSetting(LIVE_PAUSED_KEY);
    if (r.status === "ok") setPaused(r.data === true);
  }, []);

  useEffect(() => {
    void load();
    const onFocus = () => void load();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [load]);

  useIngested(() => setLastIngest(new Date()));

  const toggle = useCallback(async () => {
    const next = !paused;
    const r = await commands.setSetting(LIVE_PAUSED_KEY, next);
    if (r.status === "ok") {
      setPaused(next);
      setError(null);
    } else setError(r.error.message);
  }, [paused]);

  return { paused, lastIngest, error, toggle };
}

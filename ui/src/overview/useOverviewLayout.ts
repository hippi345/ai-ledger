import { useCallback, useEffect, useState } from "react";
import { commands } from "../bindings";
import { DEFAULT_LAYOUT, LAYOUT_KEY, parseLayout } from "./persist";
import type { Widget } from "./widgets";

/** The Overview layout, loaded from settings, saved on every change, resettable. */
export function useOverviewLayout() {
  const [layout, setLayout] = useState<Widget[]>(DEFAULT_LAYOUT);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void commands.getSetting(LAYOUT_KEY).then((r) => {
      if (r.status === "ok") setLayout(parseLayout(r.data) ?? DEFAULT_LAYOUT);
    });
  }, []);

  const change = useCallback(async (next: Widget[]) => {
    setLayout(next);
    const r = await commands.setSetting(LAYOUT_KEY, next);
    setError(r.status === "error" ? r.error.message : null);
  }, []);

  const reset = useCallback(async () => {
    setLayout(DEFAULT_LAYOUT);
    const r = await commands.resetSetting(LAYOUT_KEY);
    setError(r.status === "error" ? r.error.message : null);
  }, []);

  return { layout, error, change, reset };
}

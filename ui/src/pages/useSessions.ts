import { useCallback, useEffect, useState } from "react";
import { useSearch } from "@tanstack/react-router";
import { commands, type SessionRow_Serialize } from "../bindings";
import {
  parseFilterSearch,
  toAggFilters,
  type FilterSearch,
} from "../filters/filters";
import { parseRangeSearch, resolveRange, utcToday } from "../filters/range";
import { useIngested } from "../live/useIngested";

/** Sessions for the current range and filters, reloaded on ingest. */
export function useSessions() {
  const raw = useSearch({ strict: false }) as Record<string, unknown>;
  const key = JSON.stringify([parseRangeSearch(raw), parseFilterSearch(raw)]);
  const [sessions, setSessions] = useState<SessionRow_Serialize[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(async () => {
    const [rs, f] = JSON.parse(key) as [Record<string, unknown>, FilterSearch];
    const r = resolveRange(rs, utcToday());
    const res = await commands.listSessions(r.since, r.until, toAggFilters(f));
    if (res.status === "ok") {
      setSessions(res.data);
      setError(null);
    } else setError(res.error.message);
  }, [key]);
  useEffect(() => {
    void load();
  }, [load]);
  useIngested(() => void load());
  return { sessions, error };
}

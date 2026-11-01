import { useCallback, useEffect, useState } from "react";
import { useSearch } from "@tanstack/react-router";
import type { EChartsCoreOption } from "echarts/core";
import {
  commands,
  type ToolAggDim,
  type ToolAggMetric,
  type ToolAggQuery_Deserialize,
} from "../bindings";
import { barOption } from "../charts/breakdown";
import { stackedAreaOption } from "../charts/stackedArea";
import type { Cell } from "../export/csv";
import { parseFilterSearch, type FilterSearch } from "../filters/filters";
import {
  parseRangeSearch,
  resolveRange,
  utcToday,
  type DateRange,
} from "../filters/range";
import { useIngested } from "../live/useIngested";
import { TOP_SERIES } from "../overview/queries";

/** Label for calls without an MCP server. */
export const BUILT_IN = "Built-in";

export type ToolsData = {
  totals: { calls: number; failures: number };
  /** `[tool, mcp_server, calls, failures, failure_pct, avg_ms]` per tool and server. */
  rows: Cell[][];
  trend: EChartsCoreOption;
  /** Calls by MCP server ("" = built-in tools). */
  mcp: EChartsCoreOption;
};

type Res<T> =
  { status: "ok"; data: T } | { status: "error"; error: { message: string } };
const ok = <T>(r: Res<T>): T => {
  if (r.status === "error") throw new Error(r.error.message);
  return r.data;
};

const q = (
  r: DateRange,
  f: FilterSearch,
  groupBy: ToolAggDim[],
  metric: ToolAggMetric,
  top: number | null,
): ToolAggQuery_Deserialize => ({
  since: r.since,
  until: r.until,
  grain: "day",
  groupBy,
  app: f.app ?? [],
  mcpServer: f.mcp ?? [],
  metric,
  top,
});

/** Tools & MCP page data. Tool filters narrow the table client-side (the backend filters by app and MCP only). */
export async function loadTools(
  r: DateRange,
  f: FilterSearch,
): Promise<ToolsData> {
  const [calls, failures, duration, trend, mcp] = (
    await Promise.all([
      commands.aggregateTools(q(r, f, ["tool", "mcp_server"], "calls", null)),
      commands.aggregateTools(
        q(r, f, ["tool", "mcp_server"], "failures", null),
      ),
      commands.aggregateTools(
        q(r, f, ["tool", "mcp_server"], "duration_ms", null),
      ),
      commands.aggregateTools(q(r, f, ["bucket", "tool"], "calls", TOP_SERIES)),
      commands.aggregateTools(q(r, f, ["mcp_server"], "calls", TOP_SERIES)),
    ])
  ).map(ok);
  const k = (keys: string[]) => keys.join("\u0000");
  const fail = new Map(failures.map((x) => [k(x.keys), x.value ?? 0]));
  const dur = new Map(duration.map((x) => [k(x.keys), x.value ?? 0]));
  const only = f.tool ?? [];
  const rows = calls
    .filter((x) => only.length === 0 || only.includes(x.keys[0]))
    .map((x): Cell[] => {
      const n = x.value ?? 0;
      const fl = fail.get(k(x.keys)) ?? 0;
      const d = dur.get(k(x.keys)) ?? 0;
      return [
        x.keys[0],
        x.keys[1],
        n,
        fl,
        n ? Math.round((fl / n) * 1000) / 10 : null,
        n ? Math.round(d / n) : null,
      ];
    });
  return {
    totals: {
      calls: rows.reduce((s, r) => s + (r[2] as number), 0),
      failures: rows.reduce((s, r) => s + (r[3] as number), 0),
    },
    rows,
    trend: stackedAreaOption(trend, "tokens"),
    mcp: barOption(
      mcp.map((x) => (x.keys[0] === "" ? { ...x, keys: [BUILT_IN] } : x)),
      "tokens",
    ),
  };
}

/** loadTools for the current range and filters, reloaded on ingest. */
export function useTools() {
  const raw = useSearch({ strict: false }) as Record<string, unknown>;
  const key = JSON.stringify([parseRangeSearch(raw), parseFilterSearch(raw)]);
  const [data, setData] = useState<ToolsData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(async () => {
    const [rs, f] = JSON.parse(key) as [Record<string, unknown>, FilterSearch];
    try {
      setData(await loadTools(resolveRange(rs, utcToday()), f));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [key]);
  useEffect(() => {
    void load();
  }, [load]);
  useIngested(() => void load());
  return { data, error };
}

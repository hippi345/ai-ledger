import { useCallback, useEffect, useState } from "react";
import { useSearch } from "@tanstack/react-router";
import type { EChartsCoreOption } from "echarts/core";
import {
  commands,
  type AggMetric,
  type AggQuery_Deserialize,
} from "../bindings";
import { stackedAreaOption } from "../charts/stackedArea";
import type { Cell } from "../export/csv";
import {
  parseFilterSearch,
  toAggFilters,
  type FilterSearch,
} from "../filters/filters";
import {
  parseRangeSearch,
  resolveRange,
  utcToday,
  type DateRange,
} from "../filters/range";
import { useIngested } from "../live/useIngested";
import { TOP_SERIES } from "../overview/queries";

/** Usage dimensions with their own detail page. */
export type PageDim = "app" | "model" | "project";

export type DimensionData = {
  totals: {
    tokens: number | null;
    cost: number | null;
    requests: number | null;
  };
  /** `[key, tokens, cost_usd, requests]` per value of the dimension. */
  rows: Cell[][];
  trend: EChartsCoreOption;
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
  groupBy: AggQuery_Deserialize["groupBy"],
  metric: AggMetric,
  top: number | null,
): AggQuery_Deserialize => ({
  since: r.since,
  until: r.until,
  grain: "day",
  groupBy,
  filters: toAggFilters(f),
  metric,
  top,
  compare: false,
});

/** KPIs, a per-value table and a stacked trend for one dimension. */
export async function loadDimension(
  dim: PageDim,
  r: DateRange,
  f: FilterSearch,
): Promise<DimensionData> {
  const [tokens, cost, requests, trend] = (
    await Promise.all([
      commands.aggregate(q(r, f, [dim], "tokens", null)),
      commands.aggregate(q(r, f, [dim], "cost_usd", null)),
      commands.aggregate(q(r, f, [dim], "requests", null)),
      commands.aggregate(q(r, f, ["bucket", dim], "tokens", TOP_SERIES)),
    ])
  ).map(ok);
  const by = (rows: typeof tokens.rows) =>
    new Map(rows.map((x) => [x.keys[0], x.value]));
  const c = by(cost.rows);
  const n = by(requests.rows);
  const keys = [
    ...new Set(
      [...tokens.rows, ...cost.rows, ...requests.rows].map((x) => x.keys[0]),
    ),
  ];
  const t = by(tokens.rows);
  return {
    totals: {
      tokens: tokens.total,
      cost: cost.total,
      requests: requests.total,
    },
    rows: keys.map((k) => [
      k,
      t.get(k) ?? null,
      c.get(k) ?? null,
      n.get(k) ?? null,
    ]),
    trend: stackedAreaOption(trend.rows, "tokens"),
  };
}

/** loadDimension for the current range and filters, reloaded on ingest. */
export function useDimension(dim: PageDim) {
  const raw = useSearch({ strict: false }) as Record<string, unknown>;
  const key = JSON.stringify([parseRangeSearch(raw), parseFilterSearch(raw)]);
  const [data, setData] = useState<DimensionData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(async () => {
    const [rs, f] = JSON.parse(key) as [Record<string, unknown>, FilterSearch];
    try {
      setData(await loadDimension(dim, resolveRange(rs, utcToday()), f));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [dim, key]);
  useEffect(() => {
    void load();
  }, [load]);
  useIngested(() => void load());
  return { data, error };
}

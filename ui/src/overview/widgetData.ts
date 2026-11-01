import { useCallback, useEffect, useState } from "react";
import { useSearch } from "@tanstack/react-router";
import type { EChartsCoreOption } from "echarts/core";
import { commands } from "../bindings";
import { donutOption, barOption } from "../charts/breakdown";
import { costTrendOption } from "../charts/costTrend";
import { heatmapOption } from "../charts/heatmap";
import { stackedAreaOption } from "../charts/stackedArea";
import { toolCallsOption } from "../charts/toolCalls";
import type { Cell } from "../export/csv";
import {
  parseFilterSearch,
  toAggFilters,
  type FilterDim,
  type FilterSearch,
} from "../filters/filters";
import {
  parseRangeSearch,
  resolveRange,
  utcToday,
  type DateRange,
} from "../filters/range";
import { useIngested } from "../live/useIngested";
import {
  breakdownQuery,
  costTrendQuery,
  tokensOverTimeQuery,
  toolQuery,
} from "./queries";
import type { WidgetKind } from "./widgets";

/** A chart widget's option plus the rows behind it (for CSV export). */
export type WidgetData = {
  option: EChartsCoreOption;
  header: string[];
  rows: Cell[][];
  /** Clicking a category filters by this dimension (cross-filtering). */
  crossDim?: FilterDim;
};

type Ok<T> =
  { status: "ok"; data: T } | { status: "error"; error: { message: string } };
function ok<T>(r: Ok<T>): T {
  if (r.status === "error") throw new Error(r.error.message);
  return r.data;
}

/** Fetches and shapes one chart widget's data; `summary` has no chart (null). */
export async function loadWidget(
  kind: WidgetKind,
  range: DateRange,
  f: FilterSearch,
  today: string,
): Promise<WidgetData | null> {
  switch (kind) {
    case "summary":
      return null;
    case "tokens-over-time": {
      const { rows } = ok(
        await commands.aggregate(tokensOverTimeQuery(range, f)),
      );
      return {
        option: stackedAreaOption(rows, "tokens"),
        header: ["day", "model", "tokens"],
        rows: rows.map((r) => [r.keys[0], r.keys[1], r.value]),
        crossDim: "model",
      };
    }
    case "cost-trend": {
      const { rows } = ok(await commands.aggregate(costTrendQuery(range, f)));
      const points = rows.map((r) => ({ day: r.keys[0], cost: r.value ?? 0 }));
      return {
        option: costTrendOption(points, range.since, range.until, today),
        header: ["day", "cost_usd"],
        rows: rows.map((r) => [r.keys[0], r.value]),
      };
    }
    case "models-donut":
    case "apps-bar": {
      const dim = kind === "models-donut" ? "model" : "app";
      const { rows } = ok(
        await commands.aggregate(breakdownQuery(range, f, dim)),
      );
      return {
        option:
          kind === "models-donut"
            ? donutOption(rows, "tokens")
            : barOption(rows, "tokens"),
        header: [dim, "tokens"],
        rows: rows.map((r) => [r.keys[0], r.value]),
        crossDim: dim,
      };
    }
    case "activity-heatmap": {
      const cells = ok(
        await commands.activityHeatmap(
          range.since,
          range.until,
          toAggFilters(f),
        ),
      );
      return {
        option: heatmapOption(cells, "tokens"),
        header: ["weekday", "hour", "events", "tokens"],
        rows: cells.map((c) => [c.weekday, c.hour, c.events, c.tokens]),
      };
    }
    case "tool-calls": {
      const [calls, failures] = await Promise.all([
        commands.aggregateTools(toolQuery(range, f, "calls")),
        commands.aggregateTools(toolQuery(range, f, "failures")),
      ]);
      const c = ok(calls);
      const fl = ok(failures);
      const failed = new Map(fl.map((r) => [r.keys[0], r.value]));
      return {
        option: toolCallsOption(c, fl),
        header: ["tool", "calls", "failures"],
        rows: c.map((r) => [r.keys[0], r.value, failed.get(r.keys[0]) ?? 0]),
        crossDim: "tool",
      };
    }
  }
}

/** One widget's data for the current range and filters, reloaded on ingest. */
export function useWidgetData(kind: WidgetKind) {
  const raw = useSearch({ strict: false }) as Record<string, unknown>;
  const key = JSON.stringify([parseRangeSearch(raw), parseFilterSearch(raw)]);
  const [data, setData] = useState<WidgetData | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    const [rs, f] = JSON.parse(key) as [Record<string, unknown>, FilterSearch];
    const today = utcToday();
    try {
      setData(await loadWidget(kind, resolveRange(rs, today), f, today));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [kind, key]);

  useEffect(() => {
    void load();
  }, [load]);
  useIngested(() => void load());

  return { data, error };
}

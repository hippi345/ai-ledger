import type { EChartsCoreOption } from "echarts/core";
import { formatCost, formatTokens, seriesColor } from "./theme";

/** An aggregate row grouped by [bucket, dimension]. */
export type SeriesRow = { keys: string[]; value: number | null };

/**
 * Stacked area over time (spec §C) from rows keyed `[bucket, series]`.
 * Missing bucket/series pairs are filled with 0 so the stack stays aligned.
 */
export function stackedAreaOption(rows: SeriesRow[], metric: "tokens" | "cost"): EChartsCoreOption {
  const buckets = [...new Set(rows.map((r) => r.keys[0]))].sort();
  const totals = new Map<string, number>();
  for (const r of rows) totals.set(r.keys[1], (totals.get(r.keys[1]) ?? 0) + (r.value ?? 0));
  // Largest series first so it sits at the bottom of the stack; "Other" last.
  const names = [...totals.keys()].sort((a, b) =>
    a === "Other" ? 1 : b === "Other" ? -1 : (totals.get(b) ?? 0) - (totals.get(a) ?? 0) || a.localeCompare(b),
  );
  const at = new Map(rows.map((r) => [`${r.keys[0]}\u0000${r.keys[1]}`, r.value ?? 0]));
  const fmt = metric === "cost" ? formatCost : formatTokens;
  return {
    legend: { type: "scroll", bottom: 0 },
    xAxis: { type: "category", boundaryGap: false, data: buckets },
    yAxis: { type: "value", axisLabel: { formatter: (v: number) => fmt(v) } },
    tooltip: { trigger: "axis", valueFormatter: (v: number) => fmt(v) },
    series: names.map((name, i) => ({
      name: name === "" ? "Unknown" : name,
      type: "line",
      stack: "total",
      areaStyle: {},
      showSymbol: false,
      color: seriesColor(name, i),
      data: buckets.map((b) => at.get(`${b}\u0000${name}`) ?? 0),
    })),
  };
}

import type { EChartsCoreOption } from "echarts/core";
import type { SeriesRow } from "./stackedArea";
import { formatCost, formatTokens, seriesColor } from "./theme";

const label = (k: string) => (k === "" ? "Unknown" : k);

/** Rows keyed `[dimension]`, largest first with Other last. */
function ordered(rows: SeriesRow[]) {
  return [...rows]
    .map((r) => ({ name: r.keys[0], value: r.value ?? 0 }))
    .sort((a, b) =>
      a.name === "Other"
        ? 1
        : b.name === "Other"
          ? -1
          : b.value - a.value || a.name.localeCompare(b.name),
    );
}

/** Horizontal bar breakdown by app, model, project or MCP (spec §C). */
export function barOption(
  rows: SeriesRow[],
  metric: "tokens" | "cost",
): EChartsCoreOption {
  const items = ordered(rows).reverse(); // ECharts draws the first category at the bottom
  const fmt = metric === "cost" ? formatCost : formatTokens;
  return {
    tooltip: { trigger: "item", valueFormatter: (v: number) => fmt(v) },
    xAxis: { type: "value", axisLabel: { formatter: (v: number) => fmt(v) } },
    yAxis: { type: "category", data: items.map((i) => label(i.name)) },
    series: [
      {
        type: "bar",
        data: items.map((it, i) => ({
          value: it.value,
          itemStyle: { color: seriesColor(it.name, items.length - 1 - i) },
        })),
      },
    ],
  };
}

/** Donut breakdown; same ordering and colors as the bar. */
export function donutOption(
  rows: SeriesRow[],
  metric: "tokens" | "cost",
): EChartsCoreOption {
  const fmt = metric === "cost" ? formatCost : formatTokens;
  return {
    tooltip: { trigger: "item", valueFormatter: (v: number) => fmt(v) },
    legend: { type: "scroll", bottom: 0 },
    series: [
      {
        type: "pie",
        radius: ["45%", "70%"],
        avoidLabelOverlap: true,
        data: ordered(rows).map((it, i) => ({
          name: label(it.name),
          value: it.value,
          itemStyle: { color: seriesColor(it.name, i) },
        })),
      },
    ],
  };
}

import type { EChartsCoreOption } from "echarts/core";
import type { HeatCell } from "../bindings";
import { formatTokens } from "./theme";

const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/** Hour-of-day x day-of-week activity heatmap (spec §C), UTC hours. */
export function heatmapOption(
  cells: HeatCell[],
  metric: "events" | "tokens",
): EChartsCoreOption {
  const data = cells.map((c) => [
    c.hour,
    c.weekday,
    metric === "events" ? c.events : (c.tokens ?? 0),
  ]);
  const max = Math.max(1, ...data.map((d) => d[2]));
  return {
    tooltip: {
      trigger: "item",
      formatter: (p: { value: number[] }) =>
        `${DAYS[p.value[1]]} ${String(p.value[0]).padStart(2, "0")}:00 UTC: ${
          metric === "events" ? p.value[2] : formatTokens(p.value[2])
        }`,
    },
    xAxis: {
      type: "category",
      data: Array.from({ length: 24 }, (_, h) => String(h).padStart(2, "0")),
    },
    yAxis: { type: "category", data: DAYS, inverse: true },
    visualMap: {
      min: 0,
      max,
      show: false,
      inRange: { color: ["#141414", "#1e3a8a", "#3b82f6", "#93c5fd"] },
    },
    series: [
      {
        type: "heatmap",
        data,
        itemStyle: { borderColor: "#0a0a0a", borderWidth: 2, borderRadius: 2 },
      },
    ],
  };
}

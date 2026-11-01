import type { EChartsCoreOption } from "echarts/core";
import type { SeriesRow } from "./stackedArea";
import { PALETTE } from "./theme";

/**
 * Top-N tool calls as a horizontal bar (spec §C): successful calls plus a red
 * failures segment. `calls` and `failures` are aggregate_tools rows keyed `[tool]`.
 */
export function toolCallsOption(
  calls: SeriesRow[],
  failures: SeriesRow[],
): EChartsCoreOption {
  const failed = new Map(failures.map((r) => [r.keys[0], r.value ?? 0]));
  const items = calls
    .map((r) => ({
      tool: r.keys[0],
      calls: r.value ?? 0,
      failed: failed.get(r.keys[0]) ?? 0,
    }))
    .sort((a, b) =>
      a.tool === "Other"
        ? 1
        : b.tool === "Other"
          ? -1
          : b.calls - a.calls || a.tool.localeCompare(b.tool),
    )
    .reverse();
  return {
    tooltip: { trigger: "axis", axisPointer: { type: "shadow" } },
    legend: { bottom: 0 },
    xAxis: { type: "value", minInterval: 1 },
    yAxis: { type: "category", data: items.map((i) => i.tool) },
    series: [
      {
        name: "Succeeded",
        type: "bar",
        stack: "calls",
        color: PALETTE[0],
        data: items.map((i) => Math.max(0, i.calls - i.failed)),
      },
      {
        name: "Failed",
        type: "bar",
        stack: "calls",
        color: "#ef4444",
        data: items.map((i) => i.failed),
      },
    ],
  };
}

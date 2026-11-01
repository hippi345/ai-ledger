/** Shared chart theme: palette and number formatting (spec §C). */

/** Accent-led palette, legible on black; the last entry is reserved. */
export const PALETTE = [
  "#3b82f6",
  "#a1a1a1",
  "#60a5fa",
  "#525252",
  "#93c5fd",
  "#d4d4d4",
  "#1d4ed8",
  "#737373",
  "#404040",
];

/** Grey used for the grouped "Other" series. */
export const OTHER_COLOR = "#404040";

/** Color for the n-th series; "Other" is always grey. */
export function seriesColor(name: string, index: number): string {
  if (name === "Other") return OTHER_COLOR;
  return PALETTE[index % (PALETTE.length - 1)];
}

/** Compact tokens: 950, 12.3K, 4.2M, 1.1B (matches the summary cards). */
export function formatTokens(n: number): string {
  const v = Math.max(0, n);
  const units: [number, string][] = [
    [1e9, "B"],
    [1e6, "M"],
    [1e3, "K"],
  ];
  for (const [div, u] of units) {
    if (v >= div) {
      const x = v / div;
      const s = x >= 100 ? x.toFixed(0) : x.toFixed(1);
      return `${s.replace(/\.0$/, "")}${u}`;
    }
  }
  return v.toFixed(0);
}

/** Approximate USD: ~$0, <$0.01, ~$0.42, ~$18, ~$1,240. */
export function formatCost(usd: number): string {
  const v = Math.max(0, usd);
  if (v === 0) return "~$0";
  if (v < 0.01) return "<$0.01";
  if (v < 10) return `~$${v.toFixed(2)}`;
  return `~$${Math.round(v).toLocaleString("en-US")}`;
}

export const CHART_FONT = "Geist Sans, Geist, Inter, system-ui, sans-serif";

const axisCommon = {
  axisLine: { show: true, lineStyle: { color: "#262626" } },
  axisTick: { show: false },
  axisLabel: { color: "#737373", fontSize: 11 },
  splitLine: { show: false, lineStyle: { color: "#1f1f1f", width: 1 } },
  nameTextStyle: { color: "#737373", fontSize: 11 },
};

/**
 * Dark ECharts theme (redesign §5), registered as "ledger": quiet axes,
 * y-only split lines, dark tooltip and legend.
 */
export const LEDGER_THEME = {
  color: PALETTE,
  backgroundColor: "transparent",
  textStyle: { fontFamily: CHART_FONT, color: "#a1a1a1", fontSize: 12 },
  categoryAxis: axisCommon,
  timeAxis: axisCommon,
  logAxis: {
    ...axisCommon,
    splitLine: { show: true, lineStyle: { color: "#1f1f1f" } },
  },
  valueAxis: {
    ...axisCommon,
    axisLine: { show: false },
    splitLine: { show: true, lineStyle: { color: "#1f1f1f", width: 1 } },
  },
  legend: {
    icon: "roundRect",
    itemWidth: 8,
    itemHeight: 8,
    itemGap: 16,
    textStyle: { color: "#a1a1a1", fontSize: 12 },
    inactiveColor: "#404040",
  },
  tooltip: {
    backgroundColor: "#141414",
    borderColor: "#333333",
    borderWidth: 1,
    padding: [8, 10],
    textStyle: { color: "#ededed", fontSize: 12 },
    extraCssText: "border-radius:8px;box-shadow:0 8px 24px rgba(0,0,0,.5)",
    axisPointer: {
      lineStyle: { color: "#525252", width: 1 },
      crossStyle: { color: "#525252", width: 1 },
    },
  },
  visualMap: { textStyle: { color: "#a1a1a1" } },
};

/** Base ECharts options every chart starts from. */
export const baseOption = {
  color: PALETTE,
  backgroundColor: "transparent",
  animationDuration: 200,
  animationEasing: "cubicOut" as const,
  // Room at the bottom for legends until PR 8 moves them top-left.
  grid: { left: 8, right: 8, top: 16, bottom: 32, containLabel: true },
  tooltip: {
    trigger: "axis" as const,
    axisPointer: {
      type: "line" as const,
      lineStyle: { color: "#525252", width: 1 },
    },
  },
  textStyle: { fontFamily: CHART_FONT, color: "#a1a1a1" },
};

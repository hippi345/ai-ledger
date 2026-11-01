/** Shared chart theme: palette and number formatting (spec §C). */

export const PALETTE = [
  "#4f46e5",
  "#0ea5e9",
  "#10b981",
  "#f59e0b",
  "#ef4444",
  "#8b5cf6",
  "#14b8a6",
  "#f97316",
  "#64748b",
];

/** Color for the n-th series; "Other" is always grey. */
export function seriesColor(name: string, index: number): string {
  if (name === "Other") return "#94a3b8";
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

/** Base ECharts options every chart starts from. */
export const baseOption = {
  color: PALETTE,
  animationDuration: 300,
  grid: { left: 48, right: 16, top: 24, bottom: 32, containLabel: true },
  tooltip: { trigger: "axis" as const },
  textStyle: { fontFamily: "system-ui, sans-serif" },
};

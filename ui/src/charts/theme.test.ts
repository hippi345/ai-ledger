import {
  alpha,
  areaGradient,
  formatCost,
  formatCount,
  formatCostAxis,
  formatTokens,
  seriesColor,
  PALETTE,
} from "./theme";

test("token formatting matches the summary cards", () => {
  expect(formatTokens(950)).toBe("950");
  expect(formatTokens(12_340)).toBe("12.3K");
  expect(formatTokens(2_000_000)).toBe("2M");
  expect(formatTokens(250_000_000)).toBe("250M");
  expect(formatTokens(1_100_000_000)).toBe("1.1B");
});

test("cost formatting", () => {
  expect(formatCost(0)).toBe("~$0");
  expect(formatCost(0.004)).toBe("<$0.01");
  expect(formatCost(0.42)).toBe("~$0.42");
  expect(formatCost(1240.4)).toBe("~$1,240");
});

test("Other is grey and colors cycle", () => {
  expect(seriesColor("Other", 0)).toBe("#404040");
  expect(seriesColor("a", 0)).toBe(PALETTE[0]);
  expect(seriesColor("a", PALETTE.length - 1)).toBe(PALETTE[0]);
});

test("alpha and area gradients", () => {
  expect(alpha("#3b82f6", 0.25)).toBe("rgba(59,130,246,0.25)");
  const g = areaGradient("#3b82f6", 0.35, 0.05);
  expect(g.colorStops.map((c) => c.color)).toEqual([
    "rgba(59,130,246,0.35)",
    "rgba(59,130,246,0.05)",
  ]);
});

test("formatCount never shows -0", () => {
  expect(formatCount(-0)).toBe("0");
  expect(formatCount(-3)).toBe("0");
  expect(formatCount(1234)).toBe("1,234");
});

test("formatCostAxis keeps small ticks distinct", () => {
  expect([0, 0.002, 0.004, 0.006].map(formatCostAxis)).toEqual(["$0", "$0.002", "$0.004", "$0.006"]);
  expect(formatCostAxis(0.25)).toBe("$0.25");
  expect(formatCostAxis(1.5)).toBe("$1.5");
  expect(formatCostAxis(1240)).toBe("$1,240");
  expect(formatCostAxis(-0)).toBe("$0");
  expect(formatCostAxis(0.00001)).toBe("<$0.0001");
});

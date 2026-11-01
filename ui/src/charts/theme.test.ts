import { formatCost, formatTokens, seriesColor, PALETTE } from "./theme";

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
  expect(seriesColor("Other", 0)).toBe("#94a3b8");
  expect(seriesColor("a", 0)).toBe(PALETTE[0]);
  expect(seriesColor("a", PALETTE.length - 1)).toBe(PALETTE[0]);
});

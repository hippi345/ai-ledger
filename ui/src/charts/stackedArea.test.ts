import { stackedAreaOption } from "./stackedArea";

type S = { name: string; data: number[]; stack: string };

test("aligns series on shared buckets, largest first, Other last", () => {
  const opt = stackedAreaOption(
    [
      { keys: ["2026-11-01", "gpt-5"], value: 5 },
      { keys: ["2026-11-02", "claude"], value: 30 },
      { keys: ["2026-11-02", "Other"], value: 100 },
      { keys: ["2026-11-03", ""], value: 1 },
    ],
    "tokens",
  ) as { xAxis: { data: string[] }; series: S[] };
  expect(opt.xAxis.data).toEqual(["2026-11-01", "2026-11-02", "2026-11-03"]);
  expect(opt.series.map((s) => s.name)).toEqual(["claude", "gpt-5", "Unknown", "Other"]);
  expect(opt.series[0].data).toEqual([0, 30, 0]);
  expect(opt.series.every((s) => s.stack === "total")).toBe(true);
});

test("empty input gives an empty chart", () => {
  const opt = stackedAreaOption([], "cost") as { series: S[] };
  expect(opt.series).toEqual([]);
});

test("a single bucket shows its points so the chart is not blank", () => {
  type P = { series: { showSymbol: boolean }[] };
  const one = stackedAreaOption([{ keys: ["2026-11-02", "gpt-5"], value: 5 }], "tokens") as P;
  expect(one.series[0].showSymbol).toBe(true);
  const two = stackedAreaOption(
    [
      { keys: ["2026-11-01", "gpt-5"], value: 5 },
      { keys: ["2026-11-02", "gpt-5"], value: 6 },
    ],
    "tokens",
  ) as P;
  expect(two.series[0].showSymbol).toBe(false);
});

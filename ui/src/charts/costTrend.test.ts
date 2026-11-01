import { costTrendOption, projectTotal } from "./costTrend";

const pts = [
  { day: "2026-11-01", cost: 2 },
  { day: "2026-11-02", cost: 4 },
];

test("projects the average daily spend over the remaining days", () => {
  // 6 spent over 2 days -> 3/day; 8 days left in Nov 1..11 -> 6 + 24 = 30.
  expect(projectTotal(pts, "2026-11-01", "2026-11-11", "2026-11-02")).toBe(30);
});

test("cumulative actuals stop today and the dashed line reaches the projection", () => {
  const o = costTrendOption(pts, "2026-11-01", "2026-11-05", "2026-11-02") as {
    xAxis: { data: string[] };
    series: { data: (number | null)[]; lineStyle?: { type: string } }[];
  };
  expect(o.xAxis.data).toEqual([
    "2026-11-01",
    "2026-11-02",
    "2026-11-03",
    "2026-11-04",
  ]);
  expect(o.series[0].data).toEqual([2, 6, null, null]);
  expect(o.series[1].lineStyle?.type).toBe("dashed");
  expect(o.series[1].data).toEqual([null, 6, 9, 12]);
});

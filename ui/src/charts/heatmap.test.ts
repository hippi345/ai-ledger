import { heatmapOption } from "./heatmap";

const cells = [
  { weekday: 0, hour: 9, events: 2, tokens: 32 },
  { weekday: 6, hour: 23, events: 1, tokens: 6 },
];

test("maps cells to [hour, weekday, value] with a scaled color range", () => {
  const o = heatmapOption(cells, "tokens") as {
    xAxis: { data: string[] };
    yAxis: { data: string[] };
    visualMap: { max: number };
    series: { data: number[][] }[];
    tooltip: { formatter: (p: { value: number[] }) => string };
  };
  expect(o.xAxis.data).toHaveLength(24);
  expect(o.yAxis.data[0]).toBe("Mon");
  expect(o.series[0].data).toEqual([
    [9, 0, 32],
    [23, 6, 6],
  ]);
  expect(o.visualMap.max).toBe(32);
  expect(o.tooltip.formatter({ value: [9, 0, 32] })).toBe("Mon 09:00 UTC: 32");
});

test("empty data keeps a positive scale", () => {
  const o = heatmapOption([], "events") as { visualMap: { max: number } };
  expect(o.visualMap.max).toBe(1);
});

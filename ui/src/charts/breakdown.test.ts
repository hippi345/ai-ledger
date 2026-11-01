import { barOption, donutOption } from "./breakdown";
import { PALETTE } from "./theme";

const rows = [
  { keys: ["Other"], value: 50 },
  { keys: ["gpt-5"], value: 10 },
  { keys: ["claude"], value: 30 },
  { keys: [""], value: 5 },
];

test("bar lists largest at the top with Other at the bottom", () => {
  const o = barOption(rows, "tokens") as {
    yAxis: { data: string[] };
    series: { data: { value: number }[] }[];
  };
  expect(o.yAxis.data).toEqual(["Other", "Unknown", "gpt-5", "claude"]);
  expect(o.series[0].data.map((d) => d.value)).toEqual([50, 5, 10, 30]);
});

test("donut uses the same order and colors", () => {
  const o = donutOption(rows, "cost") as {
    series: { data: { name: string; itemStyle: { color: string } }[] }[];
  };
  const d = o.series[0].data;
  expect(d.map((x) => x.name)).toEqual(["claude", "gpt-5", "Unknown", "Other"]);
  expect(d[0].itemStyle.color).toBe(PALETTE[0]);
  expect(d[3].itemStyle.color).toBe("#404040");
});

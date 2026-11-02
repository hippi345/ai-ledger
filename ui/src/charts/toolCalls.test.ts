import { toolCallsOption } from "./toolCalls";

test("splits each tool's calls into succeeded and failed, largest on top", () => {
  const o = toolCallsOption(
    [
      { keys: ["read_file"], value: 9 },
      { keys: ["search"], value: 4 },
      { keys: ["Other"], value: 20 },
    ],
    [{ keys: ["read_file"], value: 1 }],
  ) as { yAxis: { data: string[] }; series: { name: string; data: number[] }[] };
  expect(o.yAxis.data).toEqual(["Other", "search", "read_file"]);
  expect(o.series[0].data).toEqual([20, 4, 8]);
  expect(o.series[1].data).toEqual([0, 0, 1]);
});

test("x-axis uses compact labels and few ticks", () => {
  const opt = toolCallsOption([{ keys: ["read_file"], value: 1500 }], []) as {
    xAxis: { splitNumber: number; axisLabel: { formatter: (v: number) => string } };
  };
  expect(opt.xAxis.splitNumber).toBe(3);
  expect([500, 1000, 1500].map(opt.xAxis.axisLabel.formatter)).toEqual(["500", "1K", "1.5K"]);
});

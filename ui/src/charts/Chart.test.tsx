import { render, screen } from "@testing-library/react";
import { vi } from "vitest";

const inst = {
  setOption: vi.fn(),
  dispose: vi.fn(),
  resize: vi.fn(),
  on: vi.fn(),
  off: vi.fn(),
};
vi.mock("echarts/core", () => ({ init: vi.fn(() => inst), use: vi.fn() }));
vi.mock("echarts/charts", () => ({
  BarChart: {},
  HeatmapChart: {},
  LineChart: {},
  PieChart: {},
}));
vi.mock("echarts/components", () => ({
  GridComponent: {},
  LegendComponent: {},
  TooltipComponent: {},
  VisualMapComponent: {},
}));
vi.mock("echarts/renderers", () => ({ CanvasRenderer: {} }));

import { Chart } from "./Chart";

test("applies the themed option, wires clicks, and disposes", () => {
  const onClick = vi.fn();
  const { unmount } = render(
    <Chart
      label="Tokens over time"
      option={{ series: [{ type: "line", data: [1, 2] }] }}
      onClick={onClick}
    />,
  );
  expect(
    screen.getByRole("img", { name: "Tokens over time" }),
  ).toBeInTheDocument();
  const opt = inst.setOption.mock.calls[0][0];
  expect(opt.series[0].type).toBe("line");
  expect(opt.color).toBeDefined();
  const handler = inst.on.mock.calls[0][1];
  handler({ name: "gpt-5" });
  expect(onClick).toHaveBeenCalledWith({ name: "gpt-5" });
  unmount();
  expect(inst.dispose).toHaveBeenCalled();
});

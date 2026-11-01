import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";

const inst = {
  setOption: vi.fn(),
  dispose: vi.fn(),
  resize: vi.fn(),
  on: vi.fn(),
  off: vi.fn(),
  getDataURL: vi.fn(() => "data:image/png;base64,AA"),
};
vi.mock("echarts/core", () => ({
  init: vi.fn(() => inst),
  use: vi.fn(),
  registerTheme: vi.fn(),
}));
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

const save = vi.fn();
const writeFile = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: (...a: unknown[]) => save(...a),
}));
vi.mock("@tauri-apps/plugin-fs", () => ({
  writeFile: (...a: unknown[]) => writeFile(...a),
}));

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

test("exports a PNG via the save dialog when exportName is set", async () => {
  save.mockResolvedValue("/tmp/c.png");
  render(<Chart label="Cost" option={{}} exportName="Cost trend" />);
  fireEvent.click(
    screen.getByRole("button", { name: "Export Cost trend as PNG" }),
  );
  expect(inst.getDataURL).toHaveBeenCalledWith({
    type: "png",
    pixelRatio: 2,
    backgroundColor: "#0a0a0a",
  });
  await waitFor(() => expect(writeFile).toHaveBeenCalled());
  expect(save.mock.calls[0][0].defaultPath).toMatch(
    /^ai-ledger-cost-trend-\d{4}-\d{2}-\d{2}\.png$/,
  );
});

test("no PNG button without exportName", () => {
  render(<Chart label="Cost" option={{}} />);
  expect(screen.queryByRole("button")).toBeNull();
});

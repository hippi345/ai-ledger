import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";

const toggle = vi.fn();
const useWidgetData = vi.fn();
vi.mock("./widgetData", () => ({
  useWidgetData: (k: string) => useWidgetData(k),
}));
vi.mock("../filters/useFilters", () => ({
  useCrossFilter: (dim: string) => (e: { name: string }) => toggle(dim, e.name),
}));
vi.mock("../summary/Summary", () => ({ Summary: () => <p>summary cards</p> }));
vi.mock("../charts/Chart", () => ({
  Chart: (p: {
    label: string;
    exportName?: string;
    onClick?: (e: { name: string; seriesName?: string }) => void;
  }) => (
    <button
      type="button"
      onClick={() => p.onClick?.({ name: "2026-10-30", seriesName: "gpt-5" })}
    >
      chart {p.label} {p.exportName}
    </button>
  ),
}));

import { OverviewWidget } from "./OverviewWidget";

const w = (kind: string) =>
  ({ i: `${kind}-1`, kind, x: 0, y: 0, w: 6, h: 6 }) as never;

test("summary renders the cards", () => {
  render(<OverviewWidget widget={w("summary")} />);
  expect(screen.getByText("summary cards")).toBeInTheDocument();
});

test("chart widget has CSV + PNG export and cross-filters stacked series by name", () => {
  useWidgetData.mockReturnValue({
    data: {
      option: {},
      header: ["day", "model", "tokens"],
      rows: [["2026-10-30", "gpt-5", 1]],
      crossDim: "model",
    },
    error: null,
  });
  render(<OverviewWidget widget={w("tokens-over-time")} />);
  expect(
    screen.getByRole("button", { name: "Export Tokens over time as CSV" }),
  ).toBeEnabled();
  fireEvent.click(screen.getByText(/chart Tokens over time Tokens over time/));
  expect(toggle).toHaveBeenCalledWith("model", "gpt-5");
});

test("shows errors", () => {
  useWidgetData.mockReturnValue({ data: null, error: "locked" });
  render(<OverviewWidget widget={w("apps-bar")} />);
  expect(screen.getByRole("alert")).toHaveTextContent("locked");
});

import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";

const toggle = vi.fn();
const useDimension = vi.fn();
vi.mock("./dimension", () => ({
  useDimension: (d: string) => useDimension(d),
}));
vi.mock("../filters/useFilters", () => ({
  useCrossFilter: (dim: string) => (e: { name: string }) => toggle(dim, e.name),
}));
vi.mock("../charts/Chart", () => ({
  Chart: (p: {
    label: string;
    onClick?: (e: { name: string; seriesName?: string }) => void;
  }) => (
    <button
      type="button"
      onClick={() => p.onClick?.({ name: "2026-10-30", seriesName: "cursor" })}
    >
      {p.label}
    </button>
  ),
}));

import { DimensionPage } from "./DimensionPage";

test("shows KPIs, a trend that cross-filters, and the table", () => {
  useDimension.mockReturnValue({
    data: {
      totals: { tokens: 13000, cost: 0.5, requests: 4 },
      rows: [
        ["cursor", 9000, 0.5, 3],
        ["claude", 4000, null, 1],
      ],
      trend: {},
    },
    error: null,
  });
  render(<DimensionPage dim="app" />);
  expect(useDimension).toHaveBeenCalledWith("app");
  expect(screen.getByText("Requests", { selector: "dt" }).nextSibling).toHaveTextContent("4");
  fireEvent.click(
    screen.getByRole("button", { name: "Tokens over time by app" }),
  );
  expect(toggle).toHaveBeenCalledWith("app", "cursor");
  expect(screen.getByRole("table", { name: "App usage" })).toBeInTheDocument();
  expect(screen.getAllByRole("row")).toHaveLength(3);
  expect(
    screen.getByRole("button", { name: "Export App table as CSV" }),
  ).toBeEnabled();
});

test("shows errors", () => {
  useDimension.mockReturnValue({ data: null, error: "locked" });
  render(<DimensionPage dim="model" />);
  expect(screen.getByRole("alert")).toHaveTextContent("locked");
});

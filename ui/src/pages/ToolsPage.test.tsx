import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";

const toggle = vi.fn();
const useTools = vi.fn();
vi.mock("./tools", () => ({
  BUILT_IN: "Built-in",
  useTools: () => useTools(),
}));
vi.mock("../filters/useFilters", () => ({
  useCrossFilter: (dim: string) => (e: { name: string }) => toggle(dim, e.name),
}));
vi.mock("../charts/Chart", () => ({
  Chart: (p: {
    label: string;
    onClick?: (e: { name: string; seriesName?: string }) => void;
  }) => (
    <>
      <button
        type="button"
        onClick={() => p.onClick?.({ name: "github", seriesName: "Read" })}
      >
        {p.label}
      </button>
      <button type="button" onClick={() => p.onClick?.({ name: "Built-in" })}>
        {p.label} built-in
      </button>
    </>
  ),
}));

import { ToolsPage } from "./ToolsPage";

test("totals, cross-filters and the table", () => {
  useTools.mockReturnValue({
    data: {
      totals: { calls: 6, failures: 1 },
      rows: [
        ["Read", "", 4, 0, 0, 100],
        ["search", "github", 2, 1, 50, 150],
      ],
      trend: {},
      mcp: {},
    },
    error: null,
  });
  render(<ToolsPage />);
  expect(
    screen.getByText("Failures", { selector: "dt" }).nextSibling,
  ).toHaveTextContent("1 (16.7%)");
  fireEvent.click(screen.getByRole("button", { name: "Tool calls over time" }));
  expect(toggle).toHaveBeenCalledWith("tool", "Read");
  fireEvent.click(screen.getByRole("button", { name: "Calls by MCP server" }));
  expect(toggle).toHaveBeenCalledWith("mcp", "github");
  toggle.mockClear();
  fireEvent.click(
    screen.getByRole("button", { name: "Calls by MCP server built-in" }),
  );
  expect(toggle).not.toHaveBeenCalled();
  const table = screen.getByRole("table", { name: "Tool usage" });
  expect(table).toHaveTextContent("Built-in");
  expect(table).toHaveTextContent("50%");
  expect(table).toHaveTextContent("150 ms");
});

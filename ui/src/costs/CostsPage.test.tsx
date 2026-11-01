import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";

vi.mock("./useBudget", () => ({
  useBudget: () => ({
    budget: 100,
    spent: 90,
    projected: 95,
    error: null,
    save: vi.fn(),
  }),
}));
vi.mock("../overview/widgetData", () => ({
  useWidgetData: () => ({
    data: {
      option: {},
      header: ["day", "cost_usd"],
      rows: [["2026-11-01", 1]],
    },
    error: null,
  }),
}));
vi.mock("./PriceTable", () => ({ Prices: () => <p>price table</p> }));
vi.mock("../charts/Chart", () => ({
  Chart: (p: { label: string }) => <p>{p.label}</p>,
}));

import { CostsPage } from "./CostsPage";

test("budget, trend with export, and prices", () => {
  render(<CostsPage />);
  expect(screen.getByRole("alert")).toHaveTextContent(
    "Over 80% of the budget is spent.",
  );
  expect(screen.getByText("Cost per day with projection")).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "Export Cost trend as CSV" }),
  ).toBeEnabled();
  expect(screen.getByText("price table")).toBeInTheDocument();
});

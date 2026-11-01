import { render, screen, within } from "@testing-library/react";
import { expect, test } from "vitest";
import { PriceTable } from "./PriceTable";

const row = {
  model: "opus",
  effectiveFrom: "2026-01-01",
  input: 5,
  output: 25,
  cacheRead: 0.5,
  cacheWrite: 6.25,
  cacheWrite1h: 10,
  cacheWrite12h: null,
  source: "bundled",
  estimated: ["effective_from", "cache_write_1h"],
};

test("marks estimated fields and labels overrides", () => {
  render(
    <PriceTable
      rows={[row, { ...row, source: "user", estimated: [], input: 4 }]}
    />,
  );
  const [, bundled, user] = screen.getAllByRole("row");
  const cells = within(bundled).getAllByRole("cell");
  expect(cells[1]).toHaveTextContent("2026-01-01*");
  expect(cells[2]).toHaveTextContent("$5.00");
  expect(cells[2]).not.toHaveTextContent("*");
  expect(cells[4]).toHaveTextContent("$0.500");
  expect(cells[6]).toHaveTextContent("$10.00*");
  expect(cells[7]).toHaveTextContent("—");
  expect(cells[8]).toHaveTextContent("Bundled");
  expect(within(user).getAllByRole("cell")[8]).toHaveTextContent(
    "Your override",
  );
  expect(screen.getByText(/\* Estimated/)).toBeInTheDocument();
});

test("estimated tier rates mark the source", () => {
  render(<PriceTable rows={[{ ...row, estimated: ["service_tiers"] }]} />);
  expect(
    within(screen.getAllByRole("row")[1]).getAllByRole("cell")[8],
  ).toHaveTextContent("Bundled*");
});

test("no footnote without estimates", () => {
  render(<PriceTable rows={[{ ...row, estimated: [] }]} />);
  expect(screen.queryByText(/\* Estimated/)).toBeNull();
});

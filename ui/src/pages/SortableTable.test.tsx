import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, test } from "vitest";
import { SortableTable, sortRows } from "./SortableTable";

test("sortRows: numbers, strings, nulls last, stable", () => {
  const rows = [
    ["b", 2],
    ["a", null],
    ["c", 5],
    ["d", 2],
  ];
  expect(sortRows(rows, 1, true).map((r) => r[0])).toEqual([
    "c",
    "b",
    "d",
    "a",
  ]);
  expect(sortRows(rows, 1, false).map((r) => r[0])).toEqual([
    "b",
    "d",
    "c",
    "a",
  ]);
  expect(sortRows(rows, 0, false).map((r) => r[0])).toEqual([
    "a",
    "b",
    "c",
    "d",
  ]);
});

const cols = [
  { key: "app", label: "App" },
  {
    key: "tokens",
    label: "Tokens",
    numeric: true,
    format: (v: unknown) => `${v} tok`,
  },
];
const firstCol = () =>
  screen
    .getAllByRole("row")
    .slice(1)
    .map((r) => within(r).getAllByRole("cell")[0].textContent);

test("starts sorted by the first numeric column, descending; headers toggle", () => {
  render(
    <SortableTable
      caption="Apps"
      columns={cols}
      rows={[
        ["cursor", 1],
        ["", 3],
        ["claude", 2],
      ]}
    />,
  );
  expect(firstCol()).toEqual(["Unknown", "claude", "cursor"]);
  expect(screen.getByText("3 tok")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "App" }));
  expect(firstCol()).toEqual(["Unknown", "claude", "cursor"]); // "" sorts first ascending
  fireEvent.click(screen.getByRole("button", { name: "App" }));
  expect(firstCol()).toEqual(["cursor", "claude", "Unknown"]);
  expect(screen.getByRole("columnheader", { name: "App" })).toHaveAttribute(
    "aria-sort",
    "descending",
  );
});

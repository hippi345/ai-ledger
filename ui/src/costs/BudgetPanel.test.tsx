import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { BudgetPanel } from "./BudgetPanel";

test("shows spend vs budget with the alert level", () => {
  render(
    <BudgetPanel budget={100} spent={50} projected={120} onSave={vi.fn()} />,
  );
  expect(screen.getByRole("alert")).toHaveTextContent(
    "At this pace you'll go over budget",
  );
  expect(
    screen.getByText("Budget", { selector: "dt" }).nextSibling,
  ).toHaveTextContent("~$100");
});

test("on track is a status, not an alert", () => {
  render(
    <BudgetPanel budget={100} spent={10} projected={40} onSave={vi.fn()} />,
  );
  expect(screen.queryByRole("alert")).toBeNull();
  expect(screen.getByRole("status")).toHaveTextContent("On track.");
});

test("saves a valid budget and removes it", () => {
  const onSave = vi.fn();
  const { rerender } = render(
    <BudgetPanel budget={null} spent={1} projected={2} onSave={onSave} />,
  );
  expect(screen.queryByRole("status")).toBeNull();
  const input = screen.getByLabelText(/Budget \(USD per month\)/);
  fireEvent.change(input, { target: { value: "-5" } });
  expect(screen.getByRole("button", { name: "Save budget" })).toBeDisabled();
  fireEvent.change(input, { target: { value: "75.5" } });
  fireEvent.click(screen.getByRole("button", { name: "Save budget" }));
  expect(onSave).toHaveBeenCalledWith(75.5);
  rerender(
    <BudgetPanel budget={75.5} spent={1} projected={2} onSave={onSave} />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Remove budget" }));
  expect(onSave).toHaveBeenLastCalledWith(null);
});

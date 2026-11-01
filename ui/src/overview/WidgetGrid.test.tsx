import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { WidgetGrid } from "./WidgetGrid";

test("adds the chosen widget and removes one", () => {
  const onChange = vi.fn();
  const layout = [
    { i: "summary-1", kind: "summary" as const, x: 0, y: 0, w: 12, h: 3 },
  ];
  render(
    <WidgetGrid
      layout={layout}
      onChange={onChange}
      render={(w) => <p>body {w.i}</p>}
    />,
  );
  expect(screen.getByText("body summary-1")).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Widget"), {
    target: { value: "cost-trend" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Add widget" }));
  expect(onChange.mock.calls[0][0].map((w: { i: string }) => w.i)).toEqual([
    "summary-1",
    "cost-trend-1",
  ]);
  fireEvent.click(screen.getByRole("button", { name: "Remove Summary cards" }));
  expect(onChange).toHaveBeenLastCalledWith([]);
});

test("reset button appears when resettable", () => {
  const onReset = vi.fn();
  render(
    <WidgetGrid
      layout={[]}
      onChange={vi.fn()}
      render={() => null}
      onReset={onReset}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Reset layout" }));
  expect(onReset).toHaveBeenCalled();
});

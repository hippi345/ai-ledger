import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { SavedViews } from "./SavedViews";

const views = [{ name: "Work", search: { range: "7d" as const } }];

test("saves a named view and clears the input", () => {
  const onSave = vi.fn();
  render(
    <SavedViews
      views={[]}
      onSave={onSave}
      onApply={vi.fn()}
      onDelete={vi.fn()}
    />,
  );
  const save = screen.getByRole("button", { name: "Save view" });
  expect(save).toBeDisabled();
  fireEvent.change(screen.getByLabelText("View name"), {
    target: { value: "Mine" },
  });
  fireEvent.click(save);
  expect(onSave).toHaveBeenCalledWith("Mine");
  expect(screen.getByLabelText("View name")).toHaveValue("");
});

test("applies and deletes saved views", () => {
  const onApply = vi.fn();
  const onDelete = vi.fn();
  render(
    <SavedViews
      views={views}
      onSave={vi.fn()}
      onApply={onApply}
      onDelete={onDelete}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Work" }));
  expect(onApply).toHaveBeenCalledWith(views[0]);
  fireEvent.click(screen.getByRole("button", { name: "Delete view Work" }));
  expect(onDelete).toHaveBeenCalledWith("Work");
});

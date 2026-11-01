import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { FilterBar } from "./FilterBar";

test("hidden without filters", () => {
  const { container } = render(
    <FilterBar filters={{}} onRemove={vi.fn()} onClear={vi.fn()} />,
  );
  expect(container).toBeEmptyDOMElement();
});

test("removes one chip or clears all", () => {
  const onRemove = vi.fn();
  const onClear = vi.fn();
  render(
    <FilterBar
      filters={{ app: ["cursor"], mcp: ["github"] }}
      onRemove={onRemove}
      onClear={onClear}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Remove MCP github" }));
  expect(onRemove).toHaveBeenCalledWith("mcp", "github");
  fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
  expect(onClear).toHaveBeenCalled();
});

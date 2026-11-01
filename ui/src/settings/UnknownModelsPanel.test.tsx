import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { UnknownModelsPanel } from "./UnknownModelsPanel";

const m = { model: "mystery-1", events: 3, tokens: 1200, firstSeen: "2026-10-01T00:00:00Z", lastSeen: "2026-10-30T12:00:00Z" };

test("empty state", () => {
  render(<UnknownModelsPanel models={[]} onAddPrice={vi.fn()} />);
  expect(screen.getByText(/Every model in your usage has a price/)).toBeInTheDocument();
});

test("lists unpriced models and asks to add a price", () => {
  const onAddPrice = vi.fn();
  render(<UnknownModelsPanel models={[m]} onAddPrice={onAddPrice} />);
  expect(screen.getByText("mystery-1")).toBeInTheDocument();
  expect(screen.getByText("2026-10-30")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Add price for mystery-1" }));
  expect(onAddPrice).toHaveBeenCalledWith("mystery-1");
});

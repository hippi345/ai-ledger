import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { PricesPanel } from "./PricesPanel";

const o = { model: "gpt-5", effectiveFrom: "2026-01-01", input: 1, output: 8, cacheRead: 0.1, cacheWrite: 0 };

test("empty state", () => {
  render(<PricesPanel overrides={[]} busy={false} onSave={vi.fn()} onRemove={vi.fn()} />);
  expect(screen.getByText(/No overrides/)).toBeInTheDocument();
});

test("lists overrides and removes one", () => {
  const onRemove = vi.fn();
  render(<PricesPanel overrides={[o]} busy={false} onSave={vi.fn()} onRemove={onRemove} />);
  expect(screen.getByText("gpt-5")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Remove gpt-5 from 2026-01-01" }));
  expect(onRemove).toHaveBeenCalledWith("gpt-5", "2026-01-01");
});

test("saves a typed override as numbers", () => {
  const onSave = vi.fn();
  render(<PricesPanel overrides={[]} busy={false} onSave={onSave} onRemove={vi.fn()} />);
  const type = (label: string, v: string) => fireEvent.change(screen.getByLabelText(label), { target: { value: v } });
  type("Model", " gpt-5 ");
  type("From", "2026-01-01");
  type("Input", "1");
  type("Output", "8");
  type("Cache read", "0.1");
  fireEvent.click(screen.getByRole("button", { name: "Save override" }));
  expect(onSave).toHaveBeenCalledWith(o);
});

test("busy disables actions", () => {
  render(<PricesPanel overrides={[o]} busy onSave={vi.fn()} onRemove={vi.fn()} />);
  expect(screen.getByRole("button", { name: "Save override" })).toBeDisabled();
});

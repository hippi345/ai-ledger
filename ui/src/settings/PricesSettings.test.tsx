import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { PricesSettings } from "./PricesSettings";

const o = {
  model: "gpt-5",
  effectiveFrom: "2026-01-01",
  input: 1,
  output: 8,
  cacheRead: 0,
  cacheWrite: 0,
};

test("saves and removes through the backend and reports re-pricing", async () => {
  let list: unknown[] = [];
  invoke.mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case "list_price_overrides":
        return list;
      case "set_price_override":
        list = [o];
        return 3;
      case "remove_price_override":
        list = [];
        return 1;
    }
  });
  render(<PricesSettings />);
  expect(await screen.findByText(/No overrides/)).toBeInTheDocument();
  const type = (label: string, v: string) =>
    fireEvent.change(screen.getByLabelText(label), { target: { value: v } });
  type("Model", "gpt-5");
  type("From", "2026-01-01");
  type("Input", "1");
  type("Output", "8");
  fireEvent.click(screen.getByRole("button", { name: "Save override" }));
  expect(await screen.findByRole("status")).toHaveTextContent(
    "Re-priced 3 events.",
  );
  expect(invoke).toHaveBeenCalledWith("set_price_override", { price: o });
  fireEvent.click(
    await screen.findByRole("button", { name: "Remove gpt-5 from 2026-01-01" }),
  );
  await vi.waitFor(() =>
    expect(screen.getByRole("status")).toHaveTextContent("Re-priced 1 event."),
  );
  expect(invoke).toHaveBeenCalledWith("remove_price_override", {
    model: "gpt-5",
    effectiveFrom: "2026-01-01",
  });
  expect(await screen.findByText(/No overrides/)).toBeInTheDocument();
});

test("shows backend errors", async () => {
  invoke.mockRejectedValue({
    code: "invalid_input",
    message: "rates must be non-negative numbers",
  });
  render(<PricesSettings />);
  expect(await screen.findByRole("alert")).toHaveTextContent("non-negative");
});

test("an unpriced model can be picked to prefill a new override", async () => {
  invoke.mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case "list_price_overrides":
        return [];
      case "list_unknown_models":
        return [
          {
            model: "mystery-1",
            events: 2,
            tokens: 50,
            firstSeen: "2026-10-01",
            lastSeen: "2026-10-02",
          },
        ];
    }
  });
  render(<PricesSettings />);
  fireEvent.click(
    await screen.findByRole("button", { name: "Add price for mystery-1" }),
  );
  expect(screen.getByLabelText("Model")).toHaveValue("mystery-1");
});

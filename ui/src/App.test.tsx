import { render, screen } from "@testing-library/react";
import { vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn().mockResolvedValue([]) }));

import { App } from "./App";

test("renders the app title and the settings panels", async () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: "AI Ledger" })).toBeInTheDocument();
  expect(await screen.findByRole("heading", { name: "Sources" })).toBeInTheDocument();
  expect(await screen.findByRole("heading", { name: "Price overrides" })).toBeInTheDocument();
});

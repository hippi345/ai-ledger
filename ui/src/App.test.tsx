import { fireEvent, render, screen } from "@testing-library/react";
import { createMemoryHistory } from "@tanstack/react-router";
import { vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn().mockResolvedValue([]) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));

import { App } from "./App";

const at = (path: string) => createMemoryHistory({ initialEntries: [path] });

test("sidebar lists every page and Overview is the default", async () => {
  render(<App history={at("/")} />);
  const nav = await screen.findByRole("navigation", { name: "Main" });
  expect(nav).toHaveTextContent("Tools & MCP");
  expect(screen.getAllByRole("link")).toHaveLength(8);
  expect(await screen.findByRole("heading", { name: "Overview" })).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "Overview" })).toHaveAttribute("aria-current", "page");
});

test("navigating to Settings shows the settings panels", async () => {
  render(<App history={at("/")} />);
  fireEvent.click(await screen.findByRole("link", { name: "Settings" }));
  expect(await screen.findByRole("heading", { name: "Sources" })).toBeInTheDocument();
  expect(await screen.findByRole("heading", { name: "Price overrides" })).toBeInTheDocument();
});

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createMemoryHistory } from "@tanstack/react-router";
import { vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
// ECharts needs a real canvas; chart rendering is covered in charts/.
vi.mock("./charts/Chart", () => ({ Chart: () => null }));

import { App } from "./App";

const at = (path: string) => createMemoryHistory({ initialEntries: [path] });

test("sidebar lists every page and Overview is the default", async () => {
  render(<App history={at("/")} />);
  const nav = await screen.findByRole("navigation", { name: "Main" });
  expect(nav).toHaveTextContent("Tools & MCP");
  expect(screen.getAllByRole("link")).toHaveLength(8);
  expect(
    await screen.findByRole("heading", { name: "Overview" }),
  ).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "Overview" })).toHaveAttribute(
    "aria-current",
    "page",
  );
});

test("navigating to Settings shows the settings panels", async () => {
  render(<App history={at("/")} />);
  fireEvent.click(await screen.findByRole("link", { name: "Settings" }));
  expect(
    await screen.findByRole("heading", { name: "Sources" }),
  ).toBeInTheDocument();
  expect(
    await screen.findByRole("heading", { name: "Price overrides" }),
  ).toBeInTheDocument();
});

test("the date range lives in the URL", async () => {
  const history = at("/?range=7d");
  render(<App history={history} />);
  const select = (await screen.findByLabelText("Range")) as HTMLSelectElement;
  expect(select.value).toBe("7d");
  fireEvent.change(select, { target: { value: "month" } });
  await screen.findByDisplayValue("This month");
  expect(history.location.search).toContain("range=month");
});

test("filter chips come from the URL and can be removed", async () => {
  const history = at("/?model=%5B%22gpt-5%22%5D");
  render(<App history={history} />);
  fireEvent.click(
    await screen.findByRole("button", { name: "Remove Model gpt-5" }),
  );
  await waitFor(() =>
    expect(screen.queryByRole("toolbar", { name: "Filters" })).toBeNull(),
  );
  expect(history.location.search).not.toContain("gpt-5");
});

test("the Models page shows the model usage table", async () => {
  const { invoke } = await import("@tauri-apps/api/core");
  vi.mocked(invoke).mockImplementation(async (cmd: string) =>
    cmd === "aggregate"
      ? { rows: [{ keys: ["gpt-5"], value: 5 }], total: 5 }
      : [],
  );
  render(<App history={at("/models")} />);
  expect(
    await screen.findByRole("table", { name: "Model usage" }),
  ).toHaveTextContent("gpt-5");
});

test("the Projects page shows the project usage table", async () => {
  const { invoke } = await import("@tauri-apps/api/core");
  vi.mocked(invoke).mockImplementation(async (cmd: string) =>
    cmd === "aggregate"
      ? { rows: [{ keys: ["ai-ledger"], value: 5 }], total: 5 }
      : [],
  );
  render(<App history={at("/projects")} />);
  expect(
    await screen.findByRole("table", { name: "Project usage" }),
  ).toHaveTextContent("ai-ledger");
});

test("switching pages keeps the date range and filters", async () => {
  const history = at("/?range=7d&app=%5B%22cursor%22%5D");
  render(<App history={history} />);
  fireEvent.click(await screen.findByRole("link", { name: "Models" }));
  await waitFor(() => expect(history.location.pathname).toBe("/models"));
  expect(history.location.search).toContain("range=7d");
  expect(history.location.search).toContain("cursor");
  expect(screen.getByRole("link", { name: "Models" })).toHaveAttribute(
    "aria-current",
    "page",
  );
});

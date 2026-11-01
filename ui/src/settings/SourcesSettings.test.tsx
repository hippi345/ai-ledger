import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { SourcesSettings } from "./SourcesSettings";

const src = {
  id: 1,
  readerId: "cursor",
  tier: 1,
  config: {},
  enabled: true,
  health: "unknown",
  error: null,
};
const cand = {
  readerId: "cursor",
  label: "Cursor (/c)",
  config: { root: "/c" },
  added: false,
};

test("loads, detects, adds and toggles through the backend", async () => {
  let sources: unknown[] = [];
  invoke.mockImplementation(
    async (cmd: string, args?: Record<string, unknown>) => {
      switch (cmd) {
        case "list_sources":
          return sources;
        case "detect_sources":
          return [cand];
        case "add_source":
          sources = [src];
          return 1;
        case "set_source_enabled":
          sources = [{ ...src, enabled: args?.enabled }];
          return null;
      }
    },
  );
  render(<SourcesSettings />);
  expect(await screen.findByText(/No sources yet/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Detect sources" }));
  fireEvent.click(await screen.findByRole("button", { name: "Add" }));
  const box = await screen.findByRole("checkbox", { name: "Cursor" });
  expect(invoke).toHaveBeenCalledWith("add_source", { candidate: cand });
  fireEvent.click(box);
  await vi.waitFor(() =>
    expect(screen.getByRole("checkbox", { name: "Cursor" })).not.toBeChecked(),
  );
  expect(invoke).toHaveBeenCalledWith("set_source_enabled", {
    id: 1,
    enabled: false,
  });
});

test("shows backend errors", async () => {
  invoke.mockRejectedValue({ code: "db", message: "database is locked" });
  render(<SourcesSettings />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "database is locked",
  );
});

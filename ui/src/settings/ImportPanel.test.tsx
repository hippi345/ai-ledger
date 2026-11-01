import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));
const open = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: (...a: unknown[]) => open(...a),
}));

import { ImportPanel } from "./ImportPanel";

const suggested = {
  ts: "Date",
  app: null,
  model: "Model",
  project: null,
  inputTokens: "In",
  outputTokens: null,
  cacheReadTokens: null,
  cacheWriteTokens: null,
  requests: null,
  serviceTier: null,
};
const inspection = {
  format: "csv",
  columns: ["Date", "Model", "In", "Out"],
  sample: [["2026-10-30", "gpt-5", "10", "2"]],
  rows: 1,
  suggested,
};
const preview = {
  events: 1,
  skipped: 0,
  firstTs: "2026-10-30T00:00:00Z",
  lastTs: "2026-10-30T00:00:00Z",
  inputTokens: 10,
  outputTokens: 2,
};

beforeEach(() => {
  invoke.mockReset();
  open.mockReset();
});

test("choose, adjust mapping, preview, then add", async () => {
  open.mockResolvedValue("/tmp/u.csv");
  invoke.mockImplementation(async (cmd: string) =>
    cmd === "inspect_import"
      ? inspection
      : cmd === "preview_import"
        ? preview
        : 7,
  );
  const onAdded = vi.fn();
  render(<ImportPanel onAdded={onAdded} />);
  fireEvent.click(screen.getByRole("button", { name: "Choose file…" }));
  expect(
    await screen.findByRole("table", { name: "Sample rows" }),
  ).toHaveTextContent("gpt-5");
  const out = screen.getByRole("combobox", { name: "Output tokens" });
  expect(out).toHaveValue("");
  fireEvent.change(out, { target: { value: "Out" } });
  fireEvent.change(screen.getByRole("textbox", { name: /App name/ }), {
    target: { value: "openai" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Preview" }));
  const spec = {
    path: "/tmp/u.csv",
    format: "csv",
    mapping: { ...suggested, outputTokens: "Out" },
    defaultApp: "openai",
  };
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("preview_import", { spec }),
  );
  expect(
    await screen.findByRole("status", { name: "Import preview" }),
  ).toHaveTextContent("1 events");
  fireEvent.click(screen.getByRole("button", { name: "Add source" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("add_import", { spec }),
  );
  await waitFor(() => expect(onAdded).toHaveBeenCalled());
  expect(screen.getByRole("status")).toHaveTextContent(
    "Added 1 events from /tmp/u.csv.",
  );
});

test("a cancelled dialog does nothing; backend errors are shown", async () => {
  open.mockResolvedValueOnce(null);
  render(<ImportPanel onAdded={() => {}} />);
  fireEvent.click(screen.getByRole("button", { name: "Choose file…" }));
  await waitFor(() => expect(open).toHaveBeenCalled());
  expect(invoke).not.toHaveBeenCalled();
  open.mockResolvedValueOnce("/tmp/bad.json");
  invoke.mockRejectedValue({
    code: "invalid_input",
    message: "JSON has no data/rows/items array",
  });
  fireEvent.click(screen.getByRole("button", { name: "Choose file…" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "no data/rows/items",
  );
});

test("changing the mapping clears a stale preview", async () => {
  open.mockResolvedValue("/tmp/u.csv");
  invoke.mockImplementation(async (cmd: string) =>
    cmd === "inspect_import" ? inspection : preview,
  );
  render(<ImportPanel onAdded={() => {}} />);
  fireEvent.click(screen.getByRole("button", { name: "Choose file…" }));
  await screen.findByRole("table", { name: "Sample rows" });
  fireEvent.click(screen.getByRole("button", { name: "Preview" }));
  await screen.findByRole("button", { name: "Add source" });
  fireEvent.change(screen.getByRole("combobox", { name: "Model" }), {
    target: { value: "" },
  });
  expect(screen.queryByRole("button", { name: "Add source" })).toBeNull();
});

import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { SessionTimeline } from "./SessionTimeline";

const req = {
  tsUtc: "2026-11-01T10:00:00Z",
  model: "opus",
  tokens: 11000,
  costUsd: 0.25,
  tool: null,
  mcpServer: null,
  durationMs: null,
  ok: true,
};
const call = {
  ...req,
  tokens: 0,
  costUsd: null,
  tool: "search",
  mcpServer: "github",
  durationMs: 40,
  ok: false,
};

test("shows requests and tool calls in order, and goes back", async () => {
  invoke.mockResolvedValue([req, call]);
  const onBack = vi.fn();
  render(<SessionTimeline id="s1" onBack={onBack} />);
  const items = await screen.findAllByRole("listitem");
  expect(invoke).toHaveBeenCalledWith("session_timeline", { id: "s1" });
  expect(items[0]).toHaveTextContent("10:00:00 opus · 11K tokens");
  expect(items[1]).toHaveTextContent("Tool search (github) · 40 ms · failed");
  fireEvent.click(screen.getByRole("button", { name: "← All sessions" }));
  expect(onBack).toHaveBeenCalled();
});

test("explains an empty timeline", async () => {
  invoke.mockResolvedValue([]);
  render(<SessionTimeline id="old" onBack={vi.fn()} />);
  expect(
    await screen.findByText(/retention may have pruned/),
  ).toBeInTheDocument();
});

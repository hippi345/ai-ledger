import { fireEvent, render, screen, within } from "@testing-library/react";
import { vi } from "vitest";
import type {
  Candidate,
  SourceInfo_Serialize as SourceInfo,
} from "../bindings";
import { SourcesPanel } from "./SourcesPanel";

const source = (over: Partial<SourceInfo> = {}): SourceInfo => ({
  id: 1,
  readerId: "cursor",
  tier: 1,
  config: { root: "/c" },
  enabled: true,
  health: "healthy",
  error: null,
  ...over,
});

const cand = (over: Partial<Candidate> = {}): Candidate => ({
  readerId: "claude_code",
  label: "Claude Code (~/.claude)",
  config: { root: "~/.claude" },
  added: false,
  ...over,
});

function setup(sources: SourceInfo[], candidates: Candidate[]) {
  const handlers = { onDetect: vi.fn(), onAdd: vi.fn(), onToggle: vi.fn() };
  render(
    <SourcesPanel sources={sources} candidates={candidates} {...handlers} />,
  );
  return handlers;
}

test("empty state invites detection", () => {
  const h = setup([], []);
  expect(screen.getByText(/No sources yet/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Detect sources" }));
  expect(h.onDetect).toHaveBeenCalled();
});

test("lists sources with names, tiers, and health", () => {
  setup(
    [
      source(),
      source({
        id: 2,
        readerId: "csv_import",
        tier: 2,
        health: "unhealthy",
        error: "file missing",
      }),
    ],
    [],
  );
  const list = within(screen.getByRole("list", { name: "Configured sources" }));
  expect(list.getByText("Cursor")).toBeInTheDocument();
  expect(list.getByText("Exported file")).toBeInTheDocument();
  expect(list.getByText("Error: file missing")).toBeInTheDocument();
});

test("toggling and adding call back with the right values", () => {
  const h = setup(
    [source()],
    [cand(), cand({ readerId: "cursor", label: "Cursor", added: true })],
  );
  fireEvent.click(screen.getByRole("checkbox", { name: "Cursor" }));
  expect(h.onToggle).toHaveBeenCalledWith(1, false);
  const detected = within(
    screen.getByRole("list", { name: "Detected sources" }),
  );
  expect(detected.queryByText("Cursor")).not.toBeInTheDocument();
  fireEvent.click(detected.getByRole("button", { name: "Add" }));
  expect(h.onAdd).toHaveBeenCalledWith(cand());
});

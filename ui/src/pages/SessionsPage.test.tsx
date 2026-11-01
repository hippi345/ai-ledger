import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";

const useSessions = vi.fn();
vi.mock("./useSessions", () => ({ useSessions: () => useSessions() }));

import { SessionsPage } from "./SessionsPage";

const s1 = {
  id: "s1",
  app: "claude_code",
  project: null,
  firstTs: "2026-11-01T10:00:00Z",
  lastTs: "2026-11-01T10:05:00Z",
  events: 2,
  tokens: 22000,
  costUsd: null,
  models: "haiku,opus",
};

test("lists sessions and opens one", () => {
  useSessions.mockReturnValue({ sessions: [s1], error: null });
  const onOpen = vi.fn();
  render(<SessionsPage onOpen={onOpen} />);
  const table = screen.getByRole("table", { name: "Sessions (UTC)" });
  expect(table).toHaveTextContent("Unknown");
  expect(table).toHaveTextContent("haiku,opus");
  expect(table).toHaveTextContent("22K");
  fireEvent.click(screen.getByRole("button", { name: "Open session s1" }));
  expect(onOpen).toHaveBeenCalledWith("s1");
  expect(
    screen.getByRole("button", { name: "Open session s1" }),
  ).toHaveTextContent("2026-11-01 10:05");
});

test("empty and error states", () => {
  useSessions.mockReturnValue({ sessions: [], error: null });
  const { rerender } = render(<SessionsPage />);
  expect(screen.getByText("No sessions in this range.")).toBeInTheDocument();
  useSessions.mockReturnValue({ sessions: null, error: "locked" });
  rerender(<SessionsPage />);
  expect(screen.getByRole("alert")).toHaveTextContent("locked");
});

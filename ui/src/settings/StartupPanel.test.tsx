import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { StartupPanel } from "./StartupPanel";

test("reads and sets launch at login", async () => {
  invoke.mockImplementation(async (cmd: string) =>
    cmd === "get_setting" ? true : null,
  );
  render(<StartupPanel />);
  const box = await screen.findByRole("checkbox", {
    name: "Launch AI Ledger at login",
  });
  await waitFor(() => expect(box).toBeChecked());
  fireEvent.click(box);
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("set_launch_at_login", { on: false }),
  );
  await waitFor(() => expect(box).not.toBeChecked());
});

test("shows backend errors and keeps the old value", async () => {
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === "get_setting") return false;
    throw { code: "autostart", message: "not allowed" };
  });
  render(<StartupPanel />);
  const box = await screen.findByRole("checkbox", {
    name: "Launch AI Ledger at login",
  });
  await waitFor(() => expect(box).toBeEnabled());
  fireEvent.click(box);
  expect(await screen.findByRole("alert")).toHaveTextContent("not allowed");
  expect(box).not.toBeChecked();
});

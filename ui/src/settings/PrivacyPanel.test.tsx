import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { PrivacyPanel } from "./PrivacyPanel";

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation(async (cmd: string) =>
    cmd === "get_setting" ? false : cmd === "hash_existing_projects" ? 3 : null,
  );
});

test("toggles hashing for new usage", async () => {
  render(<PrivacyPanel />);
  const box = await screen.findByRole("checkbox", {
    name: "Hash project names of new usage",
  });
  await waitFor(() => expect(box).toBeEnabled());
  fireEvent.click(box);
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("set_setting", {
      key: "privacy.hash_project_names",
      value: true,
    }),
  );
  expect(invoke).not.toHaveBeenCalledWith("hash_existing_projects");
});

test("hashing existing names needs confirmation, and cancel does nothing", async () => {
  render(<PrivacyPanel />);
  fireEvent.click(
    screen.getByRole("button", { name: "Hash existing project names…" }),
  );
  const dialog = screen.getByRole("alertdialog", {
    name: "Hash existing project names?",
  });
  expect(dialog).toHaveTextContent("can't be undone");
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.queryByRole("alertdialog")).toBeNull();
  expect(invoke).not.toHaveBeenCalledWith("hash_existing_projects");

  fireEvent.click(
    screen.getByRole("button", { name: "Hash existing project names…" }),
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Hash names permanently" }),
  );
  expect(await screen.findByRole("status")).toHaveTextContent(
    "Hashed 3 project names.",
  );
  expect(invoke).toHaveBeenCalledWith("hash_existing_projects");
});

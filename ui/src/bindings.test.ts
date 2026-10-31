import { vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a) }));

import { commands } from "./bindings";

test("appInfo invokes the app_info command", async () => {
  invoke.mockResolvedValue({ appVersion: "0.1.0", coreVersion: "0.1.0" });
  await expect(commands.appInfo()).resolves.toEqual({ appVersion: "0.1.0", coreVersion: "0.1.0" });
  expect(invoke).toHaveBeenCalledWith("app_info");
});

test("command errors come back as typed results", async () => {
  invoke.mockRejectedValue({ code: "invalid_input", message: "bad key" });
  await expect(commands.setSetting("bad key", 1)).resolves.toEqual({
    status: "error",
    error: { code: "invalid_input", message: "bad key" },
  });
  invoke.mockResolvedValue([{ key: "ui.theme", value: "dark" }]);
  await expect(commands.listSettings()).resolves.toEqual({
    status: "ok",
    data: [{ key: "ui.theme", value: "dark" }],
  });
});

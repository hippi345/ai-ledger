import { vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a) }));

import { commands } from "./bindings";

test("appInfo invokes the app_info command", async () => {
  invoke.mockResolvedValue({ appVersion: "0.1.0", coreVersion: "0.1.0" });
  await expect(commands.appInfo()).resolves.toEqual({ appVersion: "0.1.0", coreVersion: "0.1.0" });
  expect(invoke).toHaveBeenCalledWith("app_info");
});

import { act, renderHook, waitFor } from "@testing-library/react";
import { vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { useSavedViews } from "./useSavedViews";

test("loads saved views, saves and deletes", async () => {
  invoke.mockImplementation(async (cmd: string) =>
    cmd === "get_setting" ? [{ name: "Work", search: { range: "7d" } }] : null,
  );
  const { result } = renderHook(() => useSavedViews());
  await waitFor(() => expect(result.current.views).toHaveLength(1));
  await act(() => result.current.save("Mine", { app: ["cursor"] }));
  expect(invoke).toHaveBeenCalledWith("set_setting", {
    key: "ui.saved_views",
    value: [
      { name: "Work", search: { range: "7d" } },
      { name: "Mine", search: { app: ["cursor"] } },
    ],
  });
  await act(() => result.current.remove("Work"));
  expect(result.current.views).toEqual([
    { name: "Mine", search: { app: ["cursor"] } },
  ]);
});

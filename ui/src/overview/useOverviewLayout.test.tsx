import { act, renderHook, waitFor } from "@testing-library/react";
import { vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { DEFAULT_LAYOUT } from "./persist";
import { useOverviewLayout } from "./useOverviewLayout";

const saved = [
  { i: "cost-trend-1", kind: "cost-trend", x: 0, y: 0, w: 4, h: 6 },
];

test("loads the saved layout, saves changes and resets", async () => {
  invoke.mockImplementation(async (cmd: string) =>
    cmd === "get_setting" ? saved : cmd === "reset_setting" ? true : null,
  );
  const { result } = renderHook(() => useOverviewLayout());
  await waitFor(() => expect(result.current.layout).toEqual(saved));
  await act(() => result.current.change([]));
  expect(invoke).toHaveBeenCalledWith("set_setting", {
    key: "ui.overview_layout",
    value: [],
  });
  await act(() => result.current.reset());
  expect(result.current.layout).toEqual(DEFAULT_LAYOUT);
  expect(invoke).toHaveBeenCalledWith("reset_setting", {
    key: "ui.overview_layout",
  });
});

test("a corrupt saved layout falls back to the default", async () => {
  invoke.mockResolvedValue({ junk: true });
  const { result } = renderHook(() => useOverviewLayout());
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("get_setting", {
      key: "ui.overview_layout",
    }),
  );
  expect(result.current.layout).toEqual(DEFAULT_LAYOUT);
});

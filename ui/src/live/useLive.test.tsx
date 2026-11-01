import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";

const invoke = vi.fn();
let ingest: ((e: { payload: unknown }) => void) | undefined;
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_: string, cb: (e: { payload: unknown }) => void) => {
    ingest = cb;
    return () => {};
  }),
}));

import { useLive } from "./useLive";

test("loads pause, toggles it, re-reads on focus and tracks ingests", async () => {
  let stored: unknown = true;
  invoke.mockImplementation(async (cmd: string, a: { value?: unknown }) => {
    if (cmd === "get_setting") return stored;
    if (cmd === "set_setting") stored = a.value;
    return null;
  });
  const { result } = renderHook(() => useLive());
  await waitFor(() => expect(result.current.paused).toBe(true));
  await act(() => result.current.toggle());
  expect(invoke).toHaveBeenCalledWith("set_setting", {
    key: "live.paused",
    value: false,
  });
  expect(result.current.paused).toBe(false);

  stored = true; // e.g. the tray menu paused it
  act(() => void window.dispatchEvent(new Event("focus")));
  await waitFor(() => expect(result.current.paused).toBe(true));

  await waitFor(() => expect(ingest).toBeDefined());
  act(() => ingest!({ payload: { sourceId: 1, inserted: 2, updated: 0 } }));
  expect(result.current.lastIngest).toBeInstanceOf(Date);
});

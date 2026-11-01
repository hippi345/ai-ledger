import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));

import { nextMonthStart, useBudget } from "./useBudget";

test("nextMonthStart rolls the year", () => {
  expect(nextMonthStart("2026-11-15")).toBe("2026-12-01");
  expect(nextMonthStart("2026-12-31")).toBe("2027-01-01");
});

test("loads budget and month spend, saves and removes", async () => {
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === "get_setting") return 100;
    if (cmd === "aggregate")
      return { rows: [{ keys: ["2026-11-01"], value: 2 }], total: 2 };
    return null;
  });
  const { result } = renderHook(() => useBudget());
  await waitFor(() => expect(result.current.budget).toBe(100));
  await waitFor(() => expect(result.current.spent).toBe(2));
  expect(result.current.projected).toBeGreaterThanOrEqual(2);
  const q = invoke.mock.calls.find((c) => c[0] === "aggregate")![1].query;
  expect(q.filters).toEqual({ app: [], model: [], project: [] });
  expect(q.since.endsWith("-01")).toBe(true);
  await act(() => result.current.save(50));
  expect(invoke).toHaveBeenCalledWith("set_setting", {
    key: "budget.monthly_usd",
    value: 50,
  });
  await act(() => result.current.save(null));
  expect(invoke).toHaveBeenCalledWith("reset_setting", {
    key: "budget.monthly_usd",
  });
  expect(result.current.budget).toBeNull();
});

import { beforeEach, expect, test, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { loadWidget } from "./widgetData";

const r = { preset: "7d" as const, since: "2026-10-26", until: "2026-11-02" };
beforeEach(() => invoke.mockReset());

test("summary has no chart", async () => {
  expect(await loadWidget("summary", r, {}, "2026-11-01")).toBeNull();
  expect(invoke).not.toHaveBeenCalled();
});

test("tokens over time: stacked series and CSV rows, cross-filters by model", async () => {
  invoke.mockResolvedValue({
    rows: [{ keys: ["2026-10-30", "gpt-5"], value: 10 }],
    total: 10,
  });
  const d = await loadWidget(
    "tokens-over-time",
    r,
    { app: ["cursor"] },
    "2026-11-01",
  );
  expect(invoke.mock.calls[0][0]).toBe("aggregate");
  expect(invoke.mock.calls[0][1].query.filters.app).toEqual(["cursor"]);
  expect(d!.header).toEqual(["day", "model", "tokens"]);
  expect(d!.rows).toEqual([["2026-10-30", "gpt-5", 10]]);
  expect(d!.crossDim).toBe("model");
  expect((d!.option.series as unknown[]).length).toBe(1);
});

test("tool calls joins failures by tool", async () => {
  invoke.mockImplementation(
    async (_: string, a: { query: { metric: string } }) =>
      a?.query.metric === "calls"
        ? [
            { keys: ["Read"], value: 5 },
            { keys: ["Edit"], value: 2 },
          ]
        : [{ keys: ["Read"], value: 1 }],
  );
  const d = await loadWidget("tool-calls", r, {}, "2026-11-01");
  expect(d!.rows).toEqual([
    ["Read", 5, 1],
    ["Edit", 2, 0],
  ]);
});

test("backend errors are thrown with their message", async () => {
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === "aggregate") throw { kind: "db", message: "locked" };
  });
  await expect(loadWidget("apps-bar", r, {}, "2026-11-01")).rejects.toThrow(
    "locked",
  );
});

import { expect, test, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { loadDimension } from "./dimension";

const r = { preset: "7d" as const, since: "2026-10-26", until: "2026-11-02" };

test("merges tokens, cost and requests per value; keeps totals", async () => {
  invoke.mockImplementation(
    async (_: string, a: { query: { metric: string; groupBy: string[] } }) => {
      if (a?.query.groupBy.length === 2)
        return {
          rows: [{ keys: ["2026-10-30", "cursor"], value: 9 }],
          total: 9,
        };
      switch (a?.query.metric) {
        case "tokens":
          return {
            rows: [
              { keys: ["cursor"], value: 9 },
              { keys: ["claude"], value: 4 },
            ],
            total: 13,
          };
        case "cost_usd":
          return { rows: [{ keys: ["cursor"], value: 0.5 }], total: 0.5 };
        default:
          return {
            rows: [
              { keys: ["cursor"], value: 3 },
              { keys: ["claude"], value: 1 },
            ],
            total: 4,
          };
      }
    },
  );
  const d = await loadDimension("app", r, { model: ["gpt-5"] });
  expect(invoke.mock.calls[0][1].query.filters.model).toEqual(["gpt-5"]);
  expect(d.totals).toEqual({ tokens: 13, cost: 0.5, requests: 4 });
  expect(d.rows).toEqual([
    ["cursor", 9, 0.5, 3],
    ["claude", 4, null, 1],
  ]);
  expect((d.trend.series as unknown[]).length).toBe(1);
});

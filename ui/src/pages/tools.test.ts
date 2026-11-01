import { expect, test, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { loadTools } from "./tools";

const r = { preset: "7d" as const, since: "2026-10-26", until: "2026-11-02" };

test("joins calls, failures and duration per tool and server; tool filter applies", async () => {
  invoke.mockImplementation(
    async (_: string, a: { query: { metric: string; groupBy: string[] } }) => {
      const g = a?.query.groupBy.join(",");
      if (g === "bucket,tool")
        return [{ keys: ["2026-10-30", "Read"], value: 4 }];
      if (g === "mcp_server")
        return [
          { keys: [""], value: 4 },
          { keys: ["github"], value: 2 },
        ];
      switch (a?.query.metric) {
        case "calls":
          return [
            { keys: ["Read", ""], value: 4 },
            { keys: ["search", "github"], value: 2 },
          ];
        case "failures":
          return [{ keys: ["search", "github"], value: 1 }];
        default:
          return [
            { keys: ["Read", ""], value: 400 },
            { keys: ["search", "github"], value: 300 },
          ];
      }
    },
  );
  const d = await loadTools(r, { app: ["cursor"], mcp: ["github"] });
  expect(invoke.mock.calls[0][1].query).toMatchObject({
    app: ["cursor"],
    mcpServer: ["github"],
  });
  expect(d.rows).toEqual([
    ["Read", "", 4, 0, 0, 100],
    ["search", "github", 2, 1, 50, 150],
  ]);
  expect(d.totals).toEqual({ calls: 6, failures: 1 });
  const bars = d.mcp.yAxis as { data: string[] };
  expect(bars.data).toContain("Built-in");

  const only = await loadTools(r, { tool: ["search"] });
  expect(invoke.mock.calls.at(-1)![1].query.tool).toEqual(["search"]);
  expect(only.rows).toHaveLength(2); // the backend filters; the mock doesn't
});

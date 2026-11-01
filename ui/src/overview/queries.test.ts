import { expect, test } from "vitest";
import {
  breakdownQuery,
  costTrendQuery,
  tokensOverTimeQuery,
  toolQuery,
} from "./queries";

const r = { preset: "7d" as const, since: "2026-10-26", until: "2026-11-02" };
const f = {
  app: ["cursor"],
  model: ["gpt-5"],
  mcp: ["github"],
  tool: ["Read"],
};

test("usage queries carry the range and usage filters", () => {
  expect(tokensOverTimeQuery(r, f)).toEqual({
    since: "2026-10-26",
    until: "2026-11-02",
    grain: "day",
    groupBy: ["bucket", "model"],
    filters: { app: ["cursor"], model: ["gpt-5"], project: [] },
    metric: "tokens",
    top: 6,
    compare: false,
  });
  expect(costTrendQuery(r, {}).groupBy).toEqual(["bucket"]);
  expect(costTrendQuery(r, {}).metric).toBe("cost_usd");
  expect(breakdownQuery(r, {}, "app").groupBy).toEqual(["app"]);
});

test("tool queries use app and MCP filters only", () => {
  expect(toolQuery(r, f, "failures")).toEqual({
    since: "2026-10-26",
    until: "2026-11-02",
    grain: "day",
    groupBy: ["tool"],
    app: ["cursor"],
    mcpServer: ["github"],
    metric: "failures",
    top: 10,
  });
});

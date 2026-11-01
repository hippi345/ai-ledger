import type { AggQuery_Deserialize } from "../bindings";

/** Cost per day over `since <= day < until` for all usage (no filters). */
export const overviewCostQuery = (
  since: string,
  until: string,
): AggQuery_Deserialize => ({
  since,
  until,
  grain: "day",
  groupBy: ["bucket"],
  filters: { app: [], model: [], project: [] },
  metric: "cost_usd",
  top: null,
  compare: false,
});

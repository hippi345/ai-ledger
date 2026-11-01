import type {
  AggDim,
  AggMetric,
  AggQuery_Deserialize,
  ToolAggQuery_Deserialize,
  ToolAggMetric,
} from "../bindings";
import { toAggFilters, type FilterSearch } from "../filters/filters";
import type { DateRange } from "../filters/range";

/** Series kept in stacked/breakdown charts before folding the rest into "Other". */
export const TOP_SERIES = 6;
export const TOP_TOOLS = 10;

function agg(
  r: DateRange,
  f: FilterSearch,
  groupBy: AggDim[],
  metric: AggMetric,
  top: number | null,
): AggQuery_Deserialize {
  return {
    since: r.since,
    until: r.until,
    grain: "day",
    groupBy,
    filters: toAggFilters(f),
    metric,
    top,
    compare: false,
  };
}

/** Tokens per day, stacked by model. */
export const tokensOverTimeQuery = (r: DateRange, f: FilterSearch) =>
  agg(r, f, ["bucket", "model"], "tokens", TOP_SERIES);

/** Cost per day (the trend widget projects the rest of the period). */
export const costTrendQuery = (r: DateRange, f: FilterSearch) =>
  agg(r, f, ["bucket"], "cost_usd", null);

/** Tokens by one dimension, for the donut and bar breakdowns. */
export const breakdownQuery = (
  r: DateRange,
  f: FilterSearch,
  dim: "model" | "app",
) => agg(r, f, [dim], "tokens", TOP_SERIES);

/** Calls or failures per tool; app and MCP filters apply, model/project don't. */
export function toolQuery(
  r: DateRange,
  f: FilterSearch,
  metric: ToolAggMetric,
): ToolAggQuery_Deserialize {
  return {
    since: r.since,
    until: r.until,
    grain: "day",
    groupBy: ["tool"],
    app: f.app ?? [],
    mcpServer: f.mcp ?? [],
    metric,
    top: TOP_TOOLS,
  };
}

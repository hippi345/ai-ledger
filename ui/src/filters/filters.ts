/** Global filter chips (spec §D): app, model, tool, MCP server and project, kept in the URL. */

export const FILTER_DIMS = ["app", "model", "tool", "mcp", "project"] as const;
export type FilterDim = (typeof FILTER_DIMS)[number];
export type FilterSearch = Partial<Record<FilterDim, string[]>>;

export const FILTER_LABELS: Record<FilterDim, string> = {
  app: "App",
  model: "Model",
  tool: "Tool",
  mcp: "MCP",
  project: "Project",
};

/** Validates raw URL params: each dim is a list of non-empty strings (a lone string is accepted). */
export function parseFilterSearch(raw: Record<string, unknown>): FilterSearch {
  const out: FilterSearch = {};
  for (const d of FILTER_DIMS) {
    const v = raw[d];
    const list = (
      Array.isArray(v) ? v : typeof v === "string" ? [v] : []
    ).filter((x): x is string => typeof x === "string" && x.length > 0);
    if (list.length) out[d] = [...new Set(list)];
  }
  return out;
}

/** Adds `value` to `dim`, or removes it if already there. Empty dims are dropped. */
export function toggleFilter(
  f: FilterSearch,
  dim: FilterDim,
  value: string,
): FilterSearch {
  const cur = f[dim] ?? [];
  const next = cur.includes(value)
    ? cur.filter((v) => v !== value)
    : [...cur, value];
  const out = { ...f };
  if (next.length) out[dim] = next;
  else delete out[dim];
  return out;
}

/** Every active chip as `[dim, value]`, in dimension order. */
export function activeChips(f: FilterSearch): [FilterDim, string][] {
  return FILTER_DIMS.flatMap((d) =>
    (f[d] ?? []).map((v) => [d, v] as [FilterDim, string]),
  );
}

/** The usage-query filters (aggregate command); tool/MCP apply to tool queries only. */
export function toAggFilters(f: FilterSearch) {
  return { app: f.app ?? [], model: f.model ?? [], project: f.project ?? [] };
}

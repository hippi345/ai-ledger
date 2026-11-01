import { useNavigate, useSearch } from "@tanstack/react-router";
import {
  FILTER_DIMS,
  parseFilterSearch,
  toggleFilter,
  type FilterDim,
  type FilterSearch,
} from "./filters";
import { OTHER_LABEL, UNKNOWN_LABEL } from "./labels";

/** The global filters from the URL, plus setters that write them back. */
export function useFilters() {
  const filters = parseFilterSearch(
    useSearch({ strict: false }) as Record<string, unknown>,
  );
  const navigate = useNavigate();
  const set = (next: FilterSearch) =>
    void navigate({
      to: ".",
      search: (prev: Record<string, unknown>) => {
        const out: Record<string, unknown> = { ...prev };
        for (const d of FILTER_DIMS) out[d] = next[d];
        return out;
      },
    });
  return {
    filters,
    set,
    toggle: (dim: FilterDim, value: string) =>
      set(toggleFilter(filters, dim, value)),
    clear: () => set({}),
  };
}

/**
 * Cross-filtering (spec §D): maps a chart click on a category to a filter
 * value. "Other" is not a real value, so it is ignored; "Unknown" is `""`.
 */
export function clickedValue(name: string | undefined): string | null {
  if (name === undefined || name === OTHER_LABEL) return null;
  return name === UNKNOWN_LABEL ? "" : name;
}

/** A Chart `onClick` that toggles `dim` for the clicked category. */
export function useCrossFilter(dim: FilterDim) {
  const { toggle } = useFilters();
  return (p: { name: string }) => {
    const v = clickedValue(p.name);
    if (v !== null) toggle(dim, v);
  };
}

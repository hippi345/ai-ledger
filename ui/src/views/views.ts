import {
  FILTER_DIMS,
  parseFilterSearch,
  type FilterSearch,
} from "../filters/filters";
import { parseRangeSearch, type RangeSearch } from "../filters/range";

export const VIEWS_KEY = "ui.saved_views";

/** Search keys a view owns; applying a view clears all of them first. */
export const VIEW_KEYS = ["range", "from", "to", ...FILTER_DIMS] as const;

export type ViewSearch = RangeSearch & FilterSearch;
/** A named date range plus filters that can be reapplied in one click. */
export type SavedView = { name: string; search: ViewSearch };

/** Only the range and filter keys of a page's search: those are what a view captures. */
export function viewSearch(raw: Record<string, unknown>): ViewSearch {
  return { ...parseRangeSearch(raw), ...parseFilterSearch(raw) };
}

/** Saves under a trimmed name, replacing a view of the same name. */
export function saveView(
  views: SavedView[],
  name: string,
  search: ViewSearch,
): SavedView[] {
  const n = name.trim();
  if (!n) return views;
  const rest = views.filter((v) => v.name !== n);
  return [...rest, { name: n, search: viewSearch(search) }];
}

export function deleteView(views: SavedView[], name: string): SavedView[] {
  return views.filter((v) => v.name !== name);
}

/** Validates stored views; malformed entries are dropped and a non-array yields []. */
export function parseViews(raw: unknown): SavedView[] {
  if (!Array.isArray(raw)) return [];
  const out: SavedView[] = [];
  for (const v of raw) {
    if (!v || typeof v !== "object") continue;
    const { name, search } = v as Record<string, unknown>;
    if (typeof name !== "string" || !name.trim()) continue;
    if (!search || typeof search !== "object") continue;
    if (out.some((o) => o.name === name)) continue;
    out.push({ name, search: viewSearch(search as Record<string, unknown>) });
  }
  return out;
}

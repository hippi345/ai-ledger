import { WIDGET_KINDS, type Widget } from "./widgets";

/** Setting key the Overview layout is saved under. */
export const LAYOUT_KEY = "ui.overview_layout";

/**
 * The default Overview (redesign §6): summary across the top, tokens over time
 * next to cost, three breakdowns in a row, then the heatmap full width.
 * Only new and reset layouts use it; saved layouts are untouched.
 */
export const DEFAULT_LAYOUT: Widget[] = [
  { i: "summary-1", kind: "summary", x: 0, y: 0, w: 12, h: 3 },
  { i: "tokens-over-time-1", kind: "tokens-over-time", x: 0, y: 3, w: 8, h: 6 },
  { i: "cost-trend-1", kind: "cost-trend", x: 8, y: 3, w: 4, h: 6 },
  { i: "models-donut-1", kind: "models-donut", x: 0, y: 9, w: 4, h: 6 },
  { i: "apps-bar-1", kind: "apps-bar", x: 4, y: 9, w: 4, h: 6 },
  { i: "tool-calls-1", kind: "tool-calls", x: 8, y: 9, w: 4, h: 6 },
  {
    i: "activity-heatmap-1",
    kind: "activity-heatmap",
    x: 0,
    y: 15,
    w: 12,
    h: 6,
  },
];

const isInt = (v: unknown, min = 0) =>
  Number.isInteger(v) && (v as number) >= min;

/** Validates a stored layout; anything malformed gives `null` (use the default). */
export function parseLayout(raw: unknown): Widget[] | null {
  if (!Array.isArray(raw)) return null;
  const seen = new Set<string>();
  const out: Widget[] = [];
  for (const w of raw) {
    if (
      typeof w !== "object" ||
      w === null ||
      typeof w.i !== "string" ||
      seen.has(w.i) ||
      !WIDGET_KINDS.includes(w.kind) ||
      !isInt(w.x) ||
      !isInt(w.y) ||
      !isInt(w.w, 1) ||
      !isInt(w.h, 1)
    )
      return null;
    seen.add(w.i);
    out.push({ i: w.i, kind: w.kind, x: w.x, y: w.y, w: w.w, h: w.h });
  }
  return out;
}

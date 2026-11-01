import {
  WIDGET_KINDS,
  addWidget,
  type Widget,
  type WidgetKind,
} from "./widgets";

/** Setting key the Overview layout is saved under. */
export const LAYOUT_KEY = "ui.overview_layout";

/** The spec's default Overview (item 56 may refine it). */
export const DEFAULT_LAYOUT: Widget[] = (
  [
    "summary",
    "tokens-over-time",
    "cost-trend",
    "models-donut",
    "apps-bar",
    "tool-calls",
  ] as WidgetKind[]
).reduce<Widget[]>((l, k) => addWidget(l, k), []);

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

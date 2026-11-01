/** Overview widget grid model (spec §D): which widgets, and where. */

export const WIDGET_KINDS = [
  "summary",
  "tokens-over-time",
  "cost-trend",
  "models-donut",
  "apps-bar",
  "activity-heatmap",
  "tool-calls",
] as const;
export type WidgetKind = (typeof WIDGET_KINDS)[number];

export const WIDGET_LABELS: Record<WidgetKind, string> = {
  summary: "Summary cards",
  "tokens-over-time": "Tokens over time",
  "cost-trend": "Cost trend",
  "models-donut": "Models",
  "apps-bar": "Apps",
  "activity-heatmap": "Activity heatmap",
  "tool-calls": "Tool calls",
};

/** Default size in grid units (12 columns). */
const SIZES: Record<WidgetKind, { w: number; h: number }> = {
  summary: { w: 12, h: 3 },
  "tokens-over-time": { w: 8, h: 6 },
  "cost-trend": { w: 4, h: 6 },
  "models-donut": { w: 4, h: 6 },
  "apps-bar": { w: 4, h: 6 },
  "activity-heatmap": { w: 8, h: 6 },
  "tool-calls": { w: 4, h: 6 },
};

export const COLS = 12;

/** One placed widget; `i` is unique within a layout. */
export type Widget = {
  i: string;
  kind: WidgetKind;
  x: number;
  y: number;
  w: number;
  h: number;
};

/** Adds a widget of `kind` below everything else, with a fresh id. */
export function addWidget(layout: Widget[], kind: WidgetKind): Widget[] {
  const bottom = layout.reduce((m, w) => Math.max(m, w.y + w.h), 0);
  let n = 1;
  while (layout.some((w) => w.i === `${kind}-${n}`)) n++;
  return [
    ...layout,
    { i: `${kind}-${n}`, kind, x: 0, y: bottom, ...SIZES[kind] },
  ];
}

export function removeWidget(layout: Widget[], i: string): Widget[] {
  return layout.filter((w) => w.i !== i);
}

/** Grid geometry as reported by the layout engine after a drag or resize. */
export type Geometry = {
  i: string;
  x: number;
  y: number;
  w: number;
  h: number;
};

/** Copies moved/resized geometry onto the widgets; returns `layout` itself when nothing changed. */
export function applyGeometry(layout: Widget[], geo: Geometry[]): Widget[] {
  const by = new Map(geo.map((g) => [g.i, g]));
  let changed = false;
  const next = layout.map((w) => {
    const g = by.get(w.i);
    if (!g || (g.x === w.x && g.y === w.y && g.w === w.w && g.h === w.h))
      return w;
    changed = true;
    return { ...w, x: g.x, y: g.y, w: g.w, h: g.h };
  });
  return changed ? next : layout;
}

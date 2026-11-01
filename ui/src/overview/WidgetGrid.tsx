import { useState } from "react";
import GridLayout, { WidthProvider } from "react-grid-layout";
import "react-grid-layout/css/styles.css";
import "react-resizable/css/styles.css";
import {
  COLS,
  WIDGET_KINDS,
  WIDGET_LABELS,
  addWidget,
  applyGeometry,
  removeWidget,
  type Widget,
  type WidgetKind,
} from "./widgets";

type Props = {
  layout: Widget[];
  onChange: (layout: Widget[]) => void;
  render: (w: Widget) => React.ReactNode;
};

const Grid = WidthProvider(GridLayout);

/** The Overview grid (drag a widget by its title, resize from the corner): each widget with a remove button, plus an Add widget control. */
export function WidgetGrid({ layout, onChange, render }: Props) {
  const [kind, setKind] = useState<WidgetKind>("summary");
  return (
    <div>
      <div role="toolbar" aria-label="Widgets">
        <label>
          Widget{" "}
          <select
            value={kind}
            onChange={(e) => setKind(e.target.value as WidgetKind)}
          >
            {WIDGET_KINDS.map((k) => (
              <option key={k} value={k}>
                {WIDGET_LABELS[k]}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={() => onChange(addWidget(layout, kind))}>
          Add widget
        </button>
      </div>
      <Grid
        className="widget-grid"
        layout={layout.map(({ i, x, y, w, h }) => ({ i, x, y, w, h }))}
        cols={COLS}
        rowHeight={40}
        draggableHandle=".widget-handle"
        onLayoutChange={(geo) => {
          const next = applyGeometry(layout, geo);
          if (next !== layout) onChange(next);
        }}
      >
        {layout.map((w) => (
          <article key={w.i} aria-label={WIDGET_LABELS[w.kind]}>
            <header>
              <h3 className="widget-handle">{WIDGET_LABELS[w.kind]}</h3>
              <button
                type="button"
                aria-label={`Remove ${WIDGET_LABELS[w.kind]}`}
                onClick={() => onChange(removeWidget(layout, w.i))}
              >
                ×
              </button>
            </header>
            {render(w)}
          </article>
        ))}
      </Grid>
    </div>
  );
}

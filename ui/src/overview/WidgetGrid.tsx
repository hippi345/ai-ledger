import { useState } from "react";
import {
  WIDGET_KINDS,
  WIDGET_LABELS,
  addWidget,
  removeWidget,
  type Widget,
  type WidgetKind,
} from "./widgets";

type Props = {
  layout: Widget[];
  onChange: (layout: Widget[]) => void;
  render: (w: Widget) => React.ReactNode;
};

/** The Overview grid: each widget with a remove button, plus an Add widget control. */
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
      <div className="widget-grid">
        {layout.map((w) => (
          <article key={w.i} aria-label={WIDGET_LABELS[w.kind]}>
            <header>
              <h3>{WIDGET_LABELS[w.kind]}</h3>
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
      </div>
    </div>
  );
}

import { OverviewWidget } from "./OverviewWidget";
import { useOverviewLayout } from "./useOverviewLayout";
import { WidgetGrid } from "./WidgetGrid";

/** The Overview page body: the saved widget layout, editable and resettable. */
export function Overview() {
  const l = useOverviewLayout();
  return (
    <>
      {l.error && <p role="alert">{l.error}</p>}
      <WidgetGrid
        layout={l.layout}
        onChange={(next) => void l.change(next)}
        onReset={() => void l.reset()}
        render={(w) => <OverviewWidget widget={w} />}
      />
    </>
  );
}

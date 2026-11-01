import { Chart } from "../charts/Chart";
import { ExportCsvButton } from "../export/ExportCsvButton";
import { useCrossFilter } from "../filters/useFilters";
import { Summary } from "../summary/Summary";
import { useWidgetData } from "./widgetData";
import { WIDGET_LABELS, type Widget } from "./widgets";

/** Grid row height in px (matches WidgetGrid) and the space the header takes. */
const ROW_PX = 40;
const HEADER_PX = 72;

/** One Overview widget's content: its chart with CSV/PNG export, or the summary cards. */
export function OverviewWidget({ widget }: { widget: Widget }) {
  if (widget.kind === "summary") return <Summary />;
  return <ChartWidget widget={widget} />;
}

function ChartWidget({ widget }: { widget: Widget }) {
  const { data, error } = useWidgetData(widget.kind);
  const cross = useCrossFilter(data?.crossDim ?? "model");
  const label = WIDGET_LABELS[widget.kind];
  if (error) return <p role="alert">{error}</p>;
  if (!data) return <p>Loading…</p>;
  // Stacked series are categories by series name; other charts by item name.
  const byName = widget.kind === "tokens-over-time" ? "seriesName" : "name";
  return (
    <>
      <ExportCsvButton name={label} header={data.header} rows={data.rows} />
      <Chart
        label={label}
        option={data.option}
        exportName={label}
        height={Math.max(120, widget.h * ROW_PX - HEADER_PX)}
        onClick={
          data.crossDim
            ? (p) =>
                cross({
                  name: (byName === "seriesName" ? p.seriesName : p.name) ?? "",
                })
            : undefined
        }
      />
    </>
  );
}

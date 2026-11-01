import { Chart } from "../charts/Chart";
import { formatCost, formatTokens } from "../charts/theme";
import { ExportCsvButton } from "../export/ExportCsvButton";
import { FILTER_LABELS } from "../filters/filters";
import { useCrossFilter } from "../filters/useFilters";
import { useDimension, type PageDim } from "./dimension";
import { SortableTable, type Column } from "./SortableTable";

const num = (v: unknown) => (typeof v === "number" ? v : 0);

/** A detail page for apps, models or projects: KPIs, trend and a sortable table. */
export function DimensionPage({ dim }: { dim: PageDim }) {
  const { data, error } = useDimension(dim);
  const cross = useCrossFilter(dim);
  const label = FILTER_LABELS[dim];
  if (error) return <p role="alert">{error}</p>;
  if (!data) return <p>Loading…</p>;
  const columns: Column[] = [
    { key: dim, label },
    {
      key: "tokens",
      label: "Tokens",
      numeric: true,
      format: (v) => formatTokens(num(v)),
    },
    {
      key: "cost",
      label: "Cost",
      numeric: true,
      format: (v) => formatCost(num(v)),
    },
    {
      key: "requests",
      label: "Requests",
      numeric: true,
      format: (v) => num(v).toLocaleString("en-US"),
    },
  ];
  const { tokens, cost, requests } = data.totals;
  return (
    <>
      <dl className="kpis">
        <dt>Tokens</dt>
        <dd>{tokens == null ? "—" : formatTokens(tokens)}</dd>
        <dt>Cost</dt>
        <dd>{cost == null ? "—" : formatCost(cost)}</dd>
        <dt>Requests</dt>
        <dd>{requests == null ? "—" : requests.toLocaleString("en-US")}</dd>
      </dl>
      <Chart
        label={`Tokens over time by ${label.toLowerCase()}`}
        option={data.trend}
        exportName={`${label} tokens over time`}
        onClick={(p) => cross({ name: p.seriesName ?? "" })}
      />
      <ExportCsvButton
        name={`${label} table`}
        header={[dim, "tokens", "cost_usd", "requests"]}
        rows={data.rows}
      />
      <SortableTable
        caption={`${label} usage`}
        columns={columns}
        rows={data.rows}
      />
    </>
  );
}

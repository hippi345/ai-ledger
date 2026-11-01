import { Chart } from "../charts/Chart";
import { ExportCsvButton } from "../export/ExportCsvButton";
import { useCrossFilter } from "../filters/useFilters";
import { SortableTable, type Column } from "./SortableTable";
import { BUILT_IN, useTools } from "./tools";

const HEADER = [
  "tool",
  "mcp_server",
  "calls",
  "failures",
  "failure_pct",
  "avg_ms",
];

/** Tools & MCP page: call totals, calls over time, calls by MCP server, and a per-tool table. */
export function ToolsPage() {
  const { data, error } = useTools();
  const crossTool = useCrossFilter("tool");
  const crossMcp = useCrossFilter("mcp");
  if (error) return <p role="alert">{error}</p>;
  if (!data) return <p>Loading…</p>;
  const columns: Column[] = [
    { key: "tool", label: "Tool" },
    {
      key: "mcp",
      label: "MCP server",
      format: (v) => (v === "" ? BUILT_IN : String(v)),
    },
    { key: "calls", label: "Calls", numeric: true },
    { key: "failures", label: "Failures", numeric: true },
    {
      key: "rate",
      label: "Failure rate",
      numeric: true,
      format: (v) => `${v}%`,
    },
    {
      key: "avg",
      label: "Avg duration",
      numeric: true,
      format: (v) => `${v} ms`,
    },
  ];
  const rows = data.rows;
  const { calls, failures } = data.totals;
  return (
    <>
      <dl className="kpis">
        <dt>Calls</dt>
        <dd>{calls.toLocaleString("en-US")}</dd>
        <dt>Failures</dt>
        <dd>
          {failures.toLocaleString("en-US")}
          {calls > 0 && ` (${Math.round((failures / calls) * 1000) / 10}%)`}
        </dd>
      </dl>
      <Chart
        label="Tool calls over time"
        option={data.trend}
        exportName="Tool calls over time"
        onClick={(p) => crossTool({ name: p.seriesName ?? "" })}
      />
      <Chart
        label="Calls by MCP server"
        option={data.mcp}
        exportName="Calls by MCP server"
        onClick={(p) => {
          // Built-in calls have no server; that's the "" (Unknown) filter value.
          if (p.name !== BUILT_IN) crossMcp(p);
        }}
      />
      <ExportCsvButton name="Tools table" header={HEADER} rows={rows} />
      <SortableTable caption="Tool usage" columns={columns} rows={rows} />
    </>
  );
}

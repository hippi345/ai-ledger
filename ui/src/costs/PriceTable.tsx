import { useEffect, useState } from "react";
import { commands, type PriceRow_Serialize } from "../bindings";

/** Columns: label, row field, and the field name the shipped table uses in `estimated`. */
const COLS: [string, keyof PriceRow_Serialize, string][] = [
  ["Input", "input", "input"],
  ["Output", "output", "output"],
  ["Cache read", "cacheRead", "cache_read"],
  ["Cache write", "cacheWrite", "cache_write"],
  ["Cache write 1h", "cacheWrite1h", "cache_write_1h"],
  ["Cache write 12h", "cacheWrite12h", "cache_write_12h"],
];

const usd = (v: unknown) =>
  typeof v === "number" ? `$${v.toFixed(v < 1 ? 3 : 2)}` : "—";

/** Estimated values get a marker and an explanation (spec: estimates are always labelled). */
function Est({ on, children }: { on: boolean; children: React.ReactNode }) {
  return on ? (
    <span className="est" title="Estimated: not read from a vendor price page">
      {children}*
    </span>
  ) : (
    <>{children}</>
  );
}

/** The effective price table, USD per million tokens; user overrides are labelled. */
export function PriceTable({ rows }: { rows: PriceRow_Serialize[] }) {
  const anyEstimated = rows.some((r) => r.estimated.length > 0);
  return (
    <>
      <table>
        <caption>Prices (USD per million tokens)</caption>
        <thead>
          <tr>
            <th scope="col">Model</th>
            <th scope="col">From</th>
            {COLS.map(([l]) => (
              <th key={l} scope="col">
                {l}
              </th>
            ))}
            <th scope="col">Source</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={`${r.model}|${r.effectiveFrom}|${r.source}`}>
              <td>{r.model}</td>
              <td>
                <Est on={r.estimated.includes("effective_from")}>
                  {r.effectiveFrom}
                </Est>
              </td>
              {COLS.map(([l, k, est]) => (
                <td key={l} className="num">
                  <Est on={r.estimated.includes(est)}>{usd(r[k])}</Est>
                </td>
              ))}
              <td>
                <Est
                  on={
                    r.estimated.includes("service_tiers") ||
                    r.estimated.includes("long_context")
                  }
                >
                  {r.source === "user" ? "Your override" : "Bundled"}
                </Est>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {anyEstimated && (
        <p>
          * Estimated: not read from a vendor price page. On Source, it means
          the service-tier or long-context rates are estimated.
        </p>
      )}
    </>
  );
}

/** PriceTable loaded from the backend. */
export function Prices() {
  const [rows, setRows] = useState<PriceRow_Serialize[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void commands.listPrices().then((r) => {
      if (r.status === "ok") setRows(r.data);
      else setError(r.error.message);
    });
  }, []);
  if (error) return <p role="alert">{error}</p>;
  if (!rows)
    return (
      <p className="skeleton" aria-busy="true">
        <span className="sr-only">Loading…</span>
      </p>
    );
  return <PriceTable rows={rows} />;
}

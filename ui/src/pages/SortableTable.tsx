import { useState } from "react";
import type { Cell } from "../export/csv";

export type Column = {
  key: string;
  label: string;
  /** Render a cell; defaults to its string value ("" shows as Unknown). */
  format?: (v: Cell) => string;
  numeric?: boolean;
};

type Props = { caption: string; columns: Column[]; rows: Cell[][] };

/** Sorts rows by a column; nulls last either way, ties keep their order. */
export function sortRows(rows: Cell[][], col: number, desc: boolean): Cell[][] {
  return rows
    .map((r, i) => [r, i] as const)
    .sort(([a, ia], [b, ib]) => {
      const x = a[col];
      const y = b[col];
      if (x == null || y == null)
        return x == null && y == null ? ia - ib : x == null ? 1 : -1;
      const c =
        typeof x === "number" && typeof y === "number"
          ? x - y
          : String(x).localeCompare(String(y));
      return (desc ? -c : c) || ia - ib;
    })
    .map(([r]) => r);
}

const show = (v: Cell) => (v === "" ? "Unknown" : v == null ? "—" : String(v));

/** A table whose headers sort it (click again to flip); numbers sort largest first. */
export function SortableTable({ caption, columns, rows }: Props) {
  const firstNumeric = Math.max(
    0,
    columns.findIndex((c) => c.numeric),
  );
  const [sort, setSort] = useState({ col: firstNumeric, desc: true });
  const sorted = sortRows(rows, sort.col, sort.desc);
  return (
    <table>
      <caption>{caption}</caption>
      <thead>
        <tr>
          {columns.map((c, i) => (
            <th
              key={c.key}
              scope="col"
              className={c.numeric ? "num" : undefined}
              aria-sort={
                sort.col === i
                  ? sort.desc
                    ? "descending"
                    : "ascending"
                  : "none"
              }
            >
              <button
                type="button"
                onClick={() =>
                  setSort((s) =>
                    s.col === i
                      ? { col: i, desc: !s.desc }
                      : { col: i, desc: !!c.numeric },
                  )
                }
              >
                {c.label}
              </button>
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {sorted.map((r, ri) => (
          <tr key={ri}>
            {columns.map((c, i) => (
              <td key={c.key} className={c.numeric ? "num" : undefined}>
                {c.format && r[i] != null ? c.format(r[i]) : show(r[i])}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

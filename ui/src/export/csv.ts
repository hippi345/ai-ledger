export type Cell = string | number | boolean | null | undefined;

/**
 * One CSV field (RFC 4180). Text that a spreadsheet would run as a formula
 * (starting with = + - @ or a tab/CR) gets a leading apostrophe; numbers are
 * written as they are.
 */
export function csvField(v: Cell): string {
  if (v === null || v === undefined) return "";
  let s = String(v);
  if (typeof v === "string" && /^[=+\-@\t\r]/.test(s)) s = `'${s}`;
  return /[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

/** A header row plus data rows, CRLF-separated, as RFC 4180 specifies. */
export function toCsv(header: string[], rows: Cell[][]): string {
  return (
    [header, ...rows].map((r) => r.map(csvField).join(",")).join("\r\n") +
    "\r\n"
  );
}

/** A safe file name: `ai-ledger-<name>-<date>.csv`. */
export function csvFileName(name: string, date: string): string {
  const slug = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return `ai-ledger-${slug || "export"}-${date}.csv`;
}

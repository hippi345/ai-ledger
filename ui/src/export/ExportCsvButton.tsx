import { csvFileName, toCsv, type Cell } from "./csv";
import { saveCsv } from "./download";
import { utcToday } from "../filters/range";

type Props = {
  /** What is exported; used in the button label and the file name. */
  name: string;
  header: string[];
  rows: Cell[][];
};

/** Downloads the data behind a chart or table as CSV. */
export function ExportCsvButton({ name, header, rows }: Props) {
  return (
    <button
      type="button"
      aria-label={`Export ${name} as CSV`}
      disabled={rows.length === 0}
      onClick={() =>
        void saveCsv(csvFileName(name, utcToday()), toCsv(header, rows))
      }
    >
      CSV
    </button>
  );
}

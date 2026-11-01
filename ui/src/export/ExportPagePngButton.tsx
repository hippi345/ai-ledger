import type { RefObject } from "react";
import { toPng } from "html-to-image";
import { utcToday } from "../filters/range";
import { savePng } from "./download";
import { pngFileName } from "./png";

type Props = {
  /** The element to capture: usually the page section. */
  target: RefObject<HTMLElement | null>;
  name: string;
};

/** Saves the whole page (charts included) as one PNG through the save dialog. */
export function ExportPagePngButton({ target, name }: Props) {
  return (
    <button
      type="button"
      aria-label={`Export ${name} page as PNG`}
      onClick={async () => {
        if (!target.current) return;
        const url = await toPng(target.current, {
          pixelRatio: 2,
          backgroundColor: "#ffffff",
          // Don't capture the export controls themselves.
          filter: (n) =>
            !(n instanceof HTMLElement && n.dataset.noExport !== undefined),
        });
        await savePng(pngFileName(name, utcToday()), url);
      }}
      data-no-export=""
    >
      Export page PNG
    </button>
  );
}

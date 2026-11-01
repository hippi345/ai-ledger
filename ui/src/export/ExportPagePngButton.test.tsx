import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useRef } from "react";
import { expect, test, vi } from "vitest";

const toPng = vi.fn();
const save = vi.fn();
const writeFile = vi.fn();
vi.mock("html-to-image", () => ({ toPng: (...a: unknown[]) => toPng(...a) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: (...a: unknown[]) => save(...a),
}));
vi.mock("@tauri-apps/plugin-fs", () => ({
  writeFile: (...a: unknown[]) => writeFile(...a),
}));

import { ExportPagePngButton } from "./ExportPagePngButton";

function Page() {
  const ref = useRef<HTMLElement>(null);
  return (
    <section ref={ref} aria-label="Overview">
      <ExportPagePngButton target={ref} name="Overview" />
      <p>content</p>
    </section>
  );
}

test("captures the target, skipping export controls, and saves it", async () => {
  toPng.mockResolvedValue("data:image/png;base64,iVBO");
  save.mockResolvedValue("/tmp/o.png");
  render(<Page />);
  fireEvent.click(
    screen.getByRole("button", { name: "Export Overview page as PNG" }),
  );
  await waitFor(() => expect(writeFile).toHaveBeenCalled());
  const [el, opts] = toPng.mock.calls[0];
  expect(el).toBe(screen.getByRole("region", { name: "Overview" }));
  expect(opts.filter(screen.getByRole("button"))).toBe(false);
  expect(opts.filter(screen.getByText("content"))).toBe(true);
  expect(save.mock.calls[0][0].defaultPath).toMatch(
    /^ai-ledger-overview-\d{4}-\d{2}-\d{2}\.png$/,
  );
});

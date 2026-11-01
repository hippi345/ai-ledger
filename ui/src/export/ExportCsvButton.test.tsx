import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";

const save = vi.fn();
const writeFile = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: (...a: unknown[]) => save(...a),
}));
vi.mock("@tauri-apps/plugin-fs", () => ({
  writeFile: (...a: unknown[]) => writeFile(...a),
}));

import { ExportCsvButton } from "./ExportCsvButton";

test("saves the rows as a named CSV file via the save dialog", async () => {
  save.mockResolvedValue("/tmp/t.csv");
  render(
    <ExportCsvButton
      name="Tokens"
      header={["day", "tokens"]}
      rows={[["2026-10-01", 5]]}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Export Tokens as CSV" }));
  await waitFor(() => expect(writeFile).toHaveBeenCalled());
  expect(save.mock.calls[0][0].defaultPath).toMatch(
    /^ai-ledger-tokens-\d{4}-\d{2}-\d{2}\.csv$/,
  );
  expect(new TextDecoder().decode(writeFile.mock.calls[0][1])).toBe(
    "day,tokens\r\n2026-10-01,5\r\n",
  );
});

test("is disabled with no rows", () => {
  render(<ExportCsvButton name="Tokens" header={["day"]} rows={[]} />);
  expect(screen.getByRole("button")).toBeDisabled();
});

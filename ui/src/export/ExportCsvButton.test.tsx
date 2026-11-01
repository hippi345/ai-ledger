import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { ExportCsvButton } from "./ExportCsvButton";

afterEach(() => vi.restoreAllMocks());

test("downloads the rows as a named CSV file", async () => {
  const blobs: Blob[] = [];
  URL.createObjectURL = vi.fn((b: Blob) => {
    blobs.push(b);
    return "blob:x";
  });
  URL.revokeObjectURL = vi.fn();
  const click = vi
    .spyOn(HTMLAnchorElement.prototype, "click")
    .mockImplementation(function (this: HTMLAnchorElement) {
      expect(this.download).toMatch(
        /^ai-ledger-tokens-\d{4}-\d{2}-\d{2}\.csv$/,
      );
      expect(this.href).toBe("blob:x");
    });
  render(
    <ExportCsvButton
      name="Tokens"
      header={["day", "tokens"]}
      rows={[["2026-10-01", 5]]}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Export Tokens as CSV" }));
  expect(click).toHaveBeenCalledOnce();
  expect(await blobs[0].text()).toBe("day,tokens\r\n2026-10-01,5\r\n");
  expect(document.querySelector("a[download]")).toBeNull();
});

test("is disabled with no rows", () => {
  render(<ExportCsvButton name="Tokens" header={["day"]} rows={[]} />);
  expect(screen.getByRole("button")).toBeDisabled();
});

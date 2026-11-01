import { expect, test } from "vitest";
import { csvField, csvFileName, toCsv } from "./csv";

test("quotes fields with commas, quotes and newlines", () => {
  expect(csvField('a,"b"')).toBe('"a,""b"""');
  expect(csvField("x\ny")).toBe('"x\ny"');
  expect(csvField(null)).toBe("");
  expect(csvField(1.5)).toBe("1.5");
});

test("guards text that a spreadsheet would treat as a formula, not numbers", () => {
  expect(csvField("=SUM(A1)")).toBe("'=SUM(A1)");
  expect(csvField("@cmd")).toBe("'@cmd");
  expect(csvField(-3)).toBe("-3");
});

test("toCsv writes a header and CRLF rows", () => {
  expect(
    toCsv(
      ["day", "tokens"],
      [
        ["2026-10-01", 5],
        ["2026-10-02", null],
      ],
    ),
  ).toBe("day,tokens\r\n2026-10-01,5\r\n2026-10-02,\r\n");
});

test("csvFileName slugs the name", () => {
  expect(csvFileName("Tokens over time!", "2026-11-01")).toBe(
    "ai-ledger-tokens-over-time-2026-11-01.csv",
  );
  expect(csvFileName("***", "2026-11-01")).toBe(
    "ai-ledger-export-2026-11-01.csv",
  );
});

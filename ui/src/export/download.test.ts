import { beforeEach, expect, test, vi } from "vitest";

const save = vi.fn();
const writeFile = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: (...a: unknown[]) => save(...a),
}));
vi.mock("@tauri-apps/plugin-fs", () => ({
  writeFile: (...a: unknown[]) => writeFile(...a),
}));

import { dataUrlBytes, saveCsv, savePng } from "./download";

beforeEach(() => {
  save.mockReset();
  writeFile.mockReset();
});

test("saveCsv asks for a path and writes UTF-8 bytes", async () => {
  save.mockResolvedValue("/tmp/out.csv");
  expect(await saveCsv("a.csv", "é,1\r\n")).toBe(true);
  expect(save).toHaveBeenCalledWith({
    defaultPath: "a.csv",
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  const [path, bytes] = writeFile.mock.calls[0];
  expect(path).toBe("/tmp/out.csv");
  expect(new TextDecoder().decode(bytes)).toBe("é,1\r\n");
});

test("a cancelled dialog writes nothing", async () => {
  save.mockResolvedValue(null);
  expect(await saveCsv("a.csv", "x")).toBe(false);
  expect(writeFile).not.toHaveBeenCalled();
});

test("savePng decodes the data URL", async () => {
  save.mockResolvedValue("/tmp/c.png");
  expect(await savePng("c.png", "data:image/png;base64,iVBO")).toBe(true);
  expect(Array.from(writeFile.mock.calls[0][1] as Uint8Array)).toEqual([
    0x89, 0x50, 0x4e,
  ]);
});

test("dataUrlBytes rejects non-base64 URLs", () => {
  expect(() => dataUrlBytes("blob:x")).toThrow();
  expect(() => dataUrlBytes("data:text/plain,hi")).toThrow();
});

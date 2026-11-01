import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";

/**
 * Saves bytes through the native save dialog, then writes them to the chosen
 * path. Webview-style link downloads are unreliable in Tauri (macOS
 * especially), so this is the only save path. Resolves false if cancelled.
 */
export async function saveBytes(
  fileName: string,
  bytes: Uint8Array,
  filter: { name: string; extensions: string[] },
): Promise<boolean> {
  const path = await save({ defaultPath: fileName, filters: [filter] });
  if (!path) return false;
  await writeFile(path, bytes);
  return true;
}

/** Saves UTF-8 text as a CSV file. */
export function saveCsv(fileName: string, text: string): Promise<boolean> {
  return saveBytes(fileName, new TextEncoder().encode(text), {
    name: "CSV",
    extensions: ["csv"],
  });
}

/** Decodes a base64 `data:` URL (as produced by canvas exports) to bytes. */
export function dataUrlBytes(url: string): Uint8Array {
  const comma = url.indexOf(",");
  if (!url.startsWith("data:") || !url.slice(0, comma).endsWith(";base64"))
    throw new Error("expected a base64 data: URL");
  const bin = atob(url.slice(comma + 1));
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}

/** Saves a base64 PNG data URL as a PNG file. */
export function savePng(fileName: string, dataUrl: string): Promise<boolean> {
  return saveBytes(fileName, dataUrlBytes(dataUrl), {
    name: "PNG image",
    extensions: ["png"],
  });
}

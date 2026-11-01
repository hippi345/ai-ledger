import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import {
  commands,
  type ImportInspection,
  type ImportMapping,
  type ImportPreview,
  type ImportSpec,
} from "../bindings";

/** Ledger fields in the order shown, with their labels. */
export const FIELDS: { key: keyof ImportMapping; label: string }[] = [
  { key: "ts", label: "Timestamp (required)" },
  { key: "app", label: "App" },
  { key: "model", label: "Model" },
  { key: "project", label: "Project" },
  { key: "inputTokens", label: "Input tokens" },
  { key: "outputTokens", label: "Output tokens" },
  { key: "cacheReadTokens", label: "Cache read tokens" },
  { key: "cacheWriteTokens", label: "Cache write tokens" },
  { key: "requests", label: "Requests" },
  { key: "serviceTier", label: "Service tier" },
];

/** Adds a CSV/JSON usage export as a source, with a column mapping. */
export function ImportPanel({ onAdded }: { onAdded: () => void }) {
  const [path, setPath] = useState<string | null>(null);
  const [info, setInfo] = useState<ImportInspection | null>(null);
  const [mapping, setMapping] = useState<ImportMapping | null>(null);
  const [app, setApp] = useState("");
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);

  const choose = async () => {
    const p = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Usage export", extensions: ["csv", "json"] }],
    });
    if (typeof p !== "string") return;
    const r = await commands.inspectImport(p);
    setPreview(null);
    setDone(null);
    if (r.status === "ok") {
      setPath(p);
      setInfo(r.data);
      setMapping(r.data.suggested);
      setError(null);
    } else setError(r.error.message);
  };

  const spec = (): ImportSpec | null =>
    path && info && mapping
      ? { path, format: info.format, mapping, defaultApp: app }
      : null;

  const runPreview = async () => {
    const s = spec();
    if (!s) return;
    const r = await commands.previewImport(s);
    if (r.status === "ok") {
      setPreview(r.data);
      setError(null);
    } else {
      setPreview(null);
      setError(r.error.message);
    }
  };

  const add = async () => {
    const s = spec();
    if (!s) return;
    const r = await commands.addImport(s);
    if (r.status === "ok") {
      setDone(`Added ${preview?.events ?? 0} events from ${path}.`);
      setInfo(null);
      setMapping(null);
      setPreview(null);
      setError(null);
      onAdded();
    } else setError(r.error.message);
  };

  const set = (key: keyof ImportMapping, v: string) => {
    setPreview(null);
    setMapping((m) =>
      m ? { ...m, [key]: key === "ts" ? v : v === "" ? null : v } : m,
    );
  };

  return (
    <section aria-labelledby="import-h">
      <h3 id="import-h">Import a usage export (CSV or JSON)</h3>
      {error && <p role="alert">{error}</p>}
      {done && <p role="status">{done}</p>}
      <button type="button" onClick={() => void choose()}>
        Choose file…
      </button>
      {info && mapping && (
        <>
          <p>
            {path} ({info.format.toUpperCase()}, {info.rows} rows)
          </p>
          <table aria-label="Sample rows">
            <thead>
              <tr>
                {info.columns.map((c) => (
                  <th key={c}>{c}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {info.sample.map((row, i) => (
                <tr key={i}>
                  {row.map((v, j) => (
                    <td key={j}>{v}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
          <fieldset>
            <legend>Map columns</legend>
            {FIELDS.map((f) => (
              <label key={f.key} style={{ display: "block" }}>
                {f.label}{" "}
                <select
                  value={mapping[f.key] ?? ""}
                  onChange={(e) => set(f.key, e.target.value)}
                >
                  <option value="">(none)</option>
                  {info.columns.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </select>
              </label>
            ))}
            <label style={{ display: "block" }}>
              App name when no App column{" "}
              <input
                value={app}
                placeholder="e.g. openai"
                onChange={(e) => {
                  setApp(e.target.value);
                  setPreview(null);
                }}
              />
            </label>
          </fieldset>
          <button type="button" onClick={() => void runPreview()}>
            Preview
          </button>
          {preview && (
            <>
              <p role="status" aria-label="Import preview">
                {preview.events} events
                {preview.skipped > 0 && `, ${preview.skipped} rows skipped`}
                {preview.firstTs &&
                  ` from ${preview.firstTs} to ${preview.lastTs}`}
                . {preview.inputTokens.toLocaleString()} input and{" "}
                {preview.outputTokens.toLocaleString()} output tokens.
              </p>
              <button
                type="button"
                disabled={preview.events === 0}
                onClick={() => void add()}
              >
                Add source
              </button>
            </>
          )}
        </>
      )}
    </section>
  );
}

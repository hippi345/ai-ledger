/**
 * No-network audit (spec §8): the webview may talk only to the app itself.
 * Fails if the CSP allows a remote origin, a capability grants network or
 * shell access, the updater is configured, or UI code opens a network API.
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";

const root = join(__dirname, "..", "..", "..");
const conf = JSON.parse(
  readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"),
);

/** Sources allowed in any CSP directive. */
const ALLOWED = new Set([
  "'self'",
  "'unsafe-inline'",
  "data:",
  "ipc:",
  "http://ipc.localhost",
  "blob:",
]);

test("CSP allows no remote origins", () => {
  const csp: string = conf.app.security.csp;
  expect(csp).toMatch(/default-src 'self'/);
  for (const directive of csp
    .split(";")
    .map((d) => d.trim())
    .filter(Boolean)) {
    const [, ...sources] = directive.split(/\s+/);
    for (const s of sources) expect(ALLOWED, `${directive}: ${s}`).toContain(s);
  }
});

test("no updater or network/shell plugins are configured", () => {
  expect(conf.plugins?.updater).toBeUndefined();
  const pkg = JSON.parse(
    readFileSync(join(root, "ui", "package.json"), "utf8"),
  );
  const deps = Object.keys({ ...pkg.dependencies, ...pkg.devDependencies });
  for (const banned of [
    "@tauri-apps/plugin-http",
    "@tauri-apps/plugin-updater",
    "@tauri-apps/plugin-shell",
    "@tauri-apps/plugin-websocket",
    "@tauri-apps/plugin-upload",
  ])
    expect(deps).not.toContain(banned);
});

test("capabilities grant no network or shell permissions", () => {
  const dir = join(root, "src-tauri", "capabilities");
  for (const f of readdirSync(dir)) {
    const cap = JSON.parse(readFileSync(join(dir, f), "utf8"));
    for (const p of cap.permissions as (string | { identifier: string })[]) {
      const id = typeof p === "string" ? p : p.identifier;
      expect(id, f).not.toMatch(
        /^(http|shell|updater|websocket|upload|opener):/,
      );
    }
  }
});

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory()
      ? files(p)
      : /\.(ts|tsx)$/.test(n) && !/\.test\.tsx?$/.test(n)
        ? [p]
        : [];
  });
}

test("UI code uses no network APIs or remote URLs", () => {
  for (const f of files(join(root, "ui", "src"))) {
    const src = readFileSync(f, "utf8");
    expect(src, f).not.toMatch(
      /\b(fetch|XMLHttpRequest|WebSocket|EventSource|sendBeacon)\s*\(/,
    );
    expect(src, f).not.toMatch(
      /new\s+(XMLHttpRequest|WebSocket|EventSource)\b/,
    );
    expect(src, f).not.toMatch(
      /https?:\/\/(?!ipc\.localhost)[a-z0-9.-]+\.[a-z]{2,}/i,
    );
  }
});

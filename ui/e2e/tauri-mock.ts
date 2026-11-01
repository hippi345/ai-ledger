/**
 * A fake Tauri backend for the browser: installs window.__TAURI_INTERNALS__
 * so @tauri-apps/api's invoke() and listen() work without the native app.
 * Commands answer from `fixtures`; every call is recorded in window.__calls.
 */
import type { Page } from "@playwright/test";

export type Fixtures = Record<string, unknown>;

export const DEFAULT_FIXTURES: Fixtures = {
  get_setting: null,
  proxy_status: { enabled: false, port: 8788, running: false, error: null },
  set_setting: null,
  reset_setting: true,
  list_settings: [],
  list_sources: [],
  detect_sources: [],
  list_price_overrides: [],
  list_unknown_models: [],
  list_prices: [],
  summary_cards: [{ kind: "today", text: "Today: 12K tokens, ~$0.42." }],
  aggregate: {
    rows: [{ keys: ["2026-11-01", "gpt-5"], value: 1200 }],
    total: 1200,
    previousTotal: null,
    changePct: null,
  },
  aggregate_tools: [{ keys: ["Read"], value: 3 }],
  activity_heatmap: [],
  list_sessions: [],
  session_timeline: [],
  "plugin:event|listen": 1,
  "plugin:event|unlisten": null,
};

export async function mockTauri(page: Page, overrides: Fixtures = {}) {
  await page.addInitScript(
    (fixtures: Fixtures) => {
      const w = window as unknown as Record<string, unknown>;
      const calls: { cmd: string; args: unknown }[] = [];
      w.__calls = calls;
      let id = 0;
      w.__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: unknown) => {
          calls.push({ cmd, args });
          if (!(cmd in fixtures))
            throw { code: "e2e", message: `unmocked command ${cmd}` };
          const v = fixtures[cmd];
          // Per-command arrays of [matcherKey, value] could go here; keep it simple.
          return structuredClone(v);
        },
        transformCallback: () => ++id,
        unregisterCallback: () => {},
        convertFileSrc: (p: string) => p,
        metadata: {
          currentWindow: { label: "main" },
          currentWebview: { label: "main", windowLabel: "main" },
        },
      };
      w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    },
    { ...DEFAULT_FIXTURES, ...overrides },
  );
}

/** Commands the page has invoked so far. */
export async function calls(
  page: Page,
): Promise<{ cmd: string; args: unknown }[]> {
  return page.evaluate(
    () =>
      (window as unknown as { __calls: { cmd: string; args: unknown }[] })
        .__calls,
  );
}

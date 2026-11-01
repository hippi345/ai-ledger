/** Global date range (spec §D): presets or custom, as UTC `YYYY-MM-DD`, `[since, until)`. */

export const PRESETS = ["today", "7d", "30d", "month", "90d", "custom"] as const;
export type Preset = (typeof PRESETS)[number];

export type RangeSearch = { range?: Preset; from?: string; to?: string };
export type DateRange = { preset: Preset; since: string; until: string };

export const PRESET_LABELS: Record<Preset, string> = {
  today: "Today",
  "7d": "Last 7 days",
  "30d": "Last 30 days",
  month: "This month",
  "90d": "Last 90 days",
  custom: "Custom",
};

const DAY = 86_400_000;
const iso = (t: number) => new Date(t).toISOString().slice(0, 10);
const isDate = (s: unknown): s is string =>
  typeof s === "string" && /^\d{4}-\d{2}-\d{2}$/.test(s) && iso(Date.parse(`${s}T00:00:00Z`)) === s;

/** Validates raw URL search params; anything unknown falls back to defaults. */
export function parseRangeSearch(raw: Record<string, unknown>): RangeSearch {
  const range = PRESETS.includes(raw.range as Preset) ? (raw.range as Preset) : undefined;
  const out: RangeSearch = {};
  if (range) out.range = range;
  if (isDate(raw.from)) out.from = raw.from;
  if (isDate(raw.to)) out.to = raw.to;
  return out;
}

/** Resolves search params to a concrete range relative to `today` (UTC). Default: last 30 days. */
export function resolveRange(s: RangeSearch, today: string): DateRange {
  const t = Date.parse(`${today}T00:00:00Z`);
  const tomorrow = iso(t + DAY);
  switch (s.range ?? "30d") {
    case "today":
      return { preset: "today", since: today, until: tomorrow };
    case "7d":
      return { preset: "7d", since: iso(t - 6 * DAY), until: tomorrow };
    case "90d":
      return { preset: "90d", since: iso(t - 89 * DAY), until: tomorrow };
    case "month":
      return { preset: "month", since: `${today.slice(0, 8)}01`, until: tomorrow };
    case "custom":
      if (s.from && s.to && s.from <= s.to) {
        // `to` is inclusive in the UI, exclusive in queries.
        return { preset: "custom", since: s.from, until: iso(Date.parse(`${s.to}T00:00:00Z`) + DAY) };
      }
      return resolveRange({ range: "30d" }, today);
    default:
      return { preset: "30d", since: iso(t - 29 * DAY), until: tomorrow };
  }
}

/** Today's UTC date. */
export const utcToday = () => iso(Date.now());

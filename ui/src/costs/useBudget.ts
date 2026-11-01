import { useCallback, useEffect, useState } from "react";
import { commands } from "../bindings";
import { projectTotal } from "../charts/costTrend";
import { utcToday } from "../filters/range";
import { useIngested } from "../live/useIngested";
import { overviewCostQuery } from "./queries";
import { BUDGET_KEY, parseBudget } from "./budget";

/** First day of the month after `day` (UTC `YYYY-MM-DD`). */
export function nextMonthStart(day: string): string {
  const [y, m] = day.split("-").map(Number);
  return m === 12
    ? `${y + 1}-01-01`
    : `${y}-${String(m + 1).padStart(2, "0")}-01`;
}

/**
 * This month's spend (all usage, ignoring page filters: a budget is for
 * everything), its month-end projection, and the saved monthly budget.
 */
export function useBudget() {
  const [budget, setBudget] = useState<number | null>(null);
  const [spent, setSpent] = useState<number | null>(null);
  const [projected, setProjected] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadSpend = useCallback(async () => {
    const today = utcToday();
    const since = `${today.slice(0, 8)}01`;
    const until = nextMonthStart(today);
    const r = await commands.aggregate(overviewCostQuery(since, until));
    if (r.status === "error") return setError(r.error.message);
    const points = r.data.rows.map((x) => ({
      day: x.keys[0],
      cost: x.value ?? 0,
    }));
    setSpent(points.reduce((s, p) => s + p.cost, 0));
    setProjected(projectTotal(points, since, until, today));
    setError(null);
  }, []);

  useEffect(() => {
    void commands.getSetting(BUDGET_KEY).then((r) => {
      if (r.status === "ok") setBudget(parseBudget(r.data));
    });
    void loadSpend();
  }, [loadSpend]);
  useIngested(() => void loadSpend());

  const save = useCallback(async (usd: number | null) => {
    const r =
      usd === null
        ? await commands.resetSetting(BUDGET_KEY)
        : await commands.setSetting(BUDGET_KEY, usd);
    if (r.status === "error") return setError(r.error.message);
    setBudget(usd);
    setError(null);
  }, []);

  return { budget, spent, projected, error, save };
}

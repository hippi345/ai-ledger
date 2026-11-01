import { useEffect, useState } from "react";
import { formatCost } from "../charts/theme";
import { budgetLevel } from "./budget";

type Props = {
  budget: number | null;
  spent: number | null;
  projected: number | null;
  onSave: (usd: number | null) => void;
};

const MESSAGES = {
  ok: "On track.",
  near: "Over 80% of the budget is spent.",
  "projected-over": "At this pace you'll go over budget this month.",
  over: "Over budget this month.",
} as const;

/** This month's spend vs. the monthly budget, with an alert, plus the budget editor. */
export function BudgetPanel({ budget, spent, projected, onSave }: Props) {
  const [draft, setDraft] = useState(budget?.toString() ?? "");
  useEffect(() => setDraft(budget?.toString() ?? ""), [budget]);
  const value = Number(draft);
  const valid = draft.trim() !== "" && Number.isFinite(value) && value > 0;
  const level =
    budget != null && spent != null && projected != null
      ? budgetLevel(spent, projected, budget)
      : null;
  return (
    <section aria-labelledby="budget-h">
      <h3 id="budget-h">Monthly budget</h3>
      <dl className="kpis">
        <dt>Spent this month</dt>
        <dd>{spent == null ? "—" : formatCost(spent)}</dd>
        <dt>Projected</dt>
        <dd>{projected == null ? "—" : formatCost(projected)}</dd>
        <dt>Budget</dt>
        <dd>{budget == null ? "Not set" : formatCost(budget)}</dd>
      </dl>
      {level && (
        <p
          role={level === "ok" ? "status" : "alert"}
          className={`budget ${level}`}
        >
          {MESSAGES[level]}
        </p>
      )}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (valid) onSave(value);
        }}
      >
        <label>
          Budget (USD per month){" "}
          <input
            inputMode="decimal"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
          />
        </label>
        <button type="submit" disabled={!valid}>
          Save budget
        </button>
        {budget != null && (
          <button type="button" onClick={() => onSave(null)}>
            Remove budget
          </button>
        )}
      </form>
    </section>
  );
}

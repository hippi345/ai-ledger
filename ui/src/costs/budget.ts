/** Setting holding the monthly budget in USD (absent = no budget). */
export const BUDGET_KEY = "budget.monthly_usd";

/** A stored budget must be a positive, finite number. */
export function parseBudget(raw: unknown): number | null {
  return typeof raw === "number" && Number.isFinite(raw) && raw > 0
    ? raw
    : null;
}

export type BudgetLevel = "ok" | "near" | "projected-over" | "over";

/** Fraction of the budget at which "near" starts. */
export const NEAR_AT = 0.8;

/**
 * Budget alert level for this month: over once spent reaches the budget,
 * projected-over when the month-end projection exceeds it, near at 80% spent.
 */
export function budgetLevel(
  spent: number,
  projected: number,
  budget: number,
): BudgetLevel {
  if (spent >= budget) return "over";
  if (projected > budget) return "projected-over";
  if (spent >= budget * NEAR_AT) return "near";
  return "ok";
}

import { expect, test } from "vitest";
import { budgetLevel, parseBudget } from "./budget";

test("parseBudget accepts positive finite numbers only", () => {
  expect(parseBudget(50)).toBe(50);
  for (const v of [0, -1, "50", null, Infinity, NaN])
    expect(parseBudget(v)).toBeNull();
});

test("budgetLevel", () => {
  expect(budgetLevel(10, 40, 100)).toBe("ok");
  expect(budgetLevel(80, 90, 100)).toBe("near");
  expect(budgetLevel(50, 120, 100)).toBe("projected-over");
  expect(budgetLevel(100, 100, 100)).toBe("over");
});

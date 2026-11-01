import { parseRangeSearch, resolveRange } from "./range";

const today = "2026-11-04";

test("presets resolve to half-open UTC ranges", () => {
  expect(resolveRange({ range: "today" }, today)).toEqual({
    preset: "today",
    since: today,
    until: "2026-11-05",
  });
  expect(resolveRange({ range: "7d" }, today).since).toBe("2026-10-29");
  expect(resolveRange({ range: "month" }, today).since).toBe("2026-11-01");
  expect(resolveRange({}, today)).toEqual({
    preset: "30d",
    since: "2026-10-06",
    until: "2026-11-05",
  });
});

test("custom ranges include the end day and fall back when invalid", () => {
  expect(
    resolveRange(
      { range: "custom", from: "2026-10-01", to: "2026-10-31" },
      today,
    ),
  ).toEqual({
    preset: "custom",
    since: "2026-10-01",
    until: "2026-11-01",
  });
  expect(
    resolveRange(
      { range: "custom", from: "2026-10-31", to: "2026-10-01" },
      today,
    ).preset,
  ).toBe("30d");
});

test("search parsing drops junk", () => {
  expect(
    parseRangeSearch({ range: "7d", from: "2026-02-30", to: "x", other: 1 }),
  ).toEqual({ range: "7d" });
  expect(
    parseRangeSearch({ range: "custom", from: "2026-10-01", to: "2026-10-31" }),
  ).toEqual({
    range: "custom",
    from: "2026-10-01",
    to: "2026-10-31",
  });
});

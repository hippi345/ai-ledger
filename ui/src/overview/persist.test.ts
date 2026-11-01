import { DEFAULT_LAYOUT, parseLayout } from "./persist";

test("default layout starts with the summary across the top", () => {
  expect(DEFAULT_LAYOUT[0]).toMatchObject({
    kind: "summary",
    x: 0,
    y: 0,
    w: 12,
  });
  expect(new Set(DEFAULT_LAYOUT.map((w) => w.i)).size).toBe(
    DEFAULT_LAYOUT.length,
  );
});

test("round-trips a valid layout and rejects bad ones", () => {
  expect(parseLayout(JSON.parse(JSON.stringify(DEFAULT_LAYOUT)))).toEqual(
    DEFAULT_LAYOUT,
  );
  expect(parseLayout([])).toEqual([]);
  expect(parseLayout(null)).toBeNull();
  expect(
    parseLayout([{ i: "a", kind: "nope", x: 0, y: 0, w: 1, h: 1 }]),
  ).toBeNull();
  expect(
    parseLayout([{ i: "a", kind: "summary", x: 0, y: 0, w: 0, h: 1 }]),
  ).toBeNull();
  const dup = { i: "a", kind: "summary", x: 0, y: 0, w: 1, h: 1 };
  expect(parseLayout([dup, dup])).toBeNull();
});

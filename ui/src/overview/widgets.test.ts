import { addWidget, applyGeometry, removeWidget } from "./widgets";

test("adds below the lowest widget with unique ids", () => {
  let l = addWidget([], "summary");
  expect(l[0]).toEqual({
    i: "summary-1",
    kind: "summary",
    x: 0,
    y: 0,
    w: 12,
    h: 3,
  });
  l = addWidget(l, "cost-trend");
  l = addWidget(l, "cost-trend");
  expect(l.map((w) => [w.i, w.y])).toEqual([
    ["summary-1", 0],
    ["cost-trend-1", 3],
    ["cost-trend-2", 9],
  ]);
});

test("removing frees the id for reuse", () => {
  let l = addWidget(addWidget([], "apps-bar"), "apps-bar");
  l = removeWidget(l, "apps-bar-1");
  expect(l.map((w) => w.i)).toEqual(["apps-bar-2"]);
  expect(addWidget(l, "apps-bar").at(-1)?.i).toBe("apps-bar-1");
});

test("applyGeometry moves widgets and keeps identity when unchanged", () => {
  const l = addWidget(addWidget([], "summary"), "cost-trend");
  expect(applyGeometry(l, l)).toBe(l);
  const moved = applyGeometry(l, [
    { i: "cost-trend-1", x: 8, y: 0, w: 4, h: 8 },
  ]);
  expect(moved[1]).toMatchObject({
    kind: "cost-trend",
    x: 8,
    y: 0,
    w: 4,
    h: 8,
  });
  expect(moved[0]).toBe(l[0]);
});

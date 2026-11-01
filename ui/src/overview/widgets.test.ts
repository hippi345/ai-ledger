import { addWidget, removeWidget } from "./widgets";

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

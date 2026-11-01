import { expect, test } from "vitest";
import { deleteView, parseViews, saveView, viewSearch } from "./views";

test("viewSearch keeps only range and filter keys", () => {
  expect(viewSearch({ range: "7d", app: ["cursor"], junk: 1 })).toEqual({
    range: "7d",
    app: ["cursor"],
  });
});

test("saveView trims, replaces same name, ignores blank", () => {
  let v = saveView([], " Work ", { range: "7d" });
  expect(v).toEqual([{ name: "Work", search: { range: "7d" } }]);
  v = saveView(v, "Work", { range: "30d" });
  expect(v).toEqual([{ name: "Work", search: { range: "30d" } }]);
  expect(saveView(v, "  ", {})).toBe(v);
});

test("deleteView removes by name", () => {
  expect(deleteView([{ name: "a", search: {} }], "a")).toEqual([]);
});

test("parseViews drops malformed and duplicate entries", () => {
  expect(parseViews(null)).toEqual([]);
  expect(
    parseViews([
      { name: "a", search: { range: "7d" } },
      { name: "a", search: {} },
      { name: "", search: {} },
      3,
      { name: "b" },
    ]),
  ).toEqual([{ name: "a", search: { range: "7d" } }]);
});

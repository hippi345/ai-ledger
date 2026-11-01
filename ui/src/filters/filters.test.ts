import {
  activeChips,
  parseFilterSearch,
  toAggFilters,
  toggleFilter,
} from "./filters";

test("parses lists, single values and drops junk", () => {
  expect(
    parseFilterSearch({
      app: ["cursor", "cursor", ""],
      model: "gpt-5",
      tool: 3,
      x: ["y"],
    }),
  ).toEqual({
    app: ["cursor"],
    model: ["gpt-5"],
  });
});

test("toggling adds, removes and drops empty dims", () => {
  let f = toggleFilter({}, "model", "gpt-5");
  f = toggleFilter(f, "model", "claude");
  expect(f).toEqual({ model: ["gpt-5", "claude"] });
  f = toggleFilter(toggleFilter(f, "model", "gpt-5"), "model", "claude");
  expect(f).toEqual({});
});

test("chips and query filters", () => {
  const f = { project: ["web"], app: ["cursor"], mcp: ["github"] };
  expect(activeChips(f)).toEqual([
    ["app", "cursor"],
    ["mcp", "github"],
    ["project", "web"],
  ]);
  expect(toAggFilters(f)).toEqual({
    app: ["cursor"],
    model: [],
    project: ["web"],
  });
});

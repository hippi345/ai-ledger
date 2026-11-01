import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRouter,
} from "@tanstack/react-router";
import { clickedValue, useCrossFilter, useFilters } from "./useFilters";

test("chart categories map to filter values", () => {
  expect(clickedValue("gpt-5")).toBe("gpt-5");
  expect(clickedValue("Unknown")).toBe("");
  expect(clickedValue("Other")).toBeNull();
  expect(clickedValue(undefined)).toBeNull();
});

function Probe() {
  const { filters } = useFilters();
  const onClick = useCrossFilter("model");
  return (
    <>
      <output>{JSON.stringify(filters)}</output>
      <button onClick={() => onClick({ name: "gpt-5" })}>gpt-5</button>
      <button onClick={() => onClick({ name: "Other" })}>Other</button>
    </>
  );
}

test("clicking a chart category toggles the URL filter", async () => {
  const history = createMemoryHistory({ initialEntries: ["/"] });
  const router = createRouter({
    routeTree: createRootRoute({ component: Probe }),
    history,
  });
  render(<RouterProvider router={router} />);
  fireEvent.click(await screen.findByText("gpt-5"));
  await waitFor(() =>
    expect(screen.getByRole("status")).toHaveTextContent('{"model":["gpt-5"]}'),
  );
  fireEvent.click(screen.getByText("Other"));
  fireEvent.click(screen.getByText("gpt-5"));
  await waitFor(() =>
    expect(screen.getByRole("status")).toHaveTextContent("{}"),
  );
});

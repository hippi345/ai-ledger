import { render, screen } from "@testing-library/react";
import { SummaryCards } from "./SummaryCards";

test("renders one card per period", () => {
  render(
    <SummaryCards
      cards={[
        { label: "Today", text: "Today: no usage yet." },
        {
          label: "This week",
          text: "This week: 4.2M tokens, ~$18, mostly cursor + gpt-5, up 30%.",
        },
      ]}
    />,
  );
  expect(screen.getByRole("article", { name: "This week" })).toHaveTextContent(
    "up 30%",
  );
  expect(screen.getAllByRole("article")).toHaveLength(2);
});

test("shows a loading state before the first load", () => {
  render(<SummaryCards cards={[]} loading />);
  expect(screen.getByText(/Loading summary/)).toBeInTheDocument();
});

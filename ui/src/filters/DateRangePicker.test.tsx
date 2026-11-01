import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { DateRangePicker } from "./DateRangePicker";

const v30 = {
  preset: "30d" as const,
  since: "2026-10-06",
  until: "2026-11-05",
};

test("choosing a preset reports just the preset", () => {
  const onChange = vi.fn();
  render(<DateRangePicker value={v30} search={{}} onChange={onChange} />);
  fireEvent.change(screen.getByLabelText("Range"), { target: { value: "7d" } });
  expect(onChange).toHaveBeenCalledWith({ range: "7d" });
});

test("custom starts from the current range and edits dates", () => {
  const onChange = vi.fn();
  render(<DateRangePicker value={v30} search={{}} onChange={onChange} />);
  fireEvent.change(screen.getByLabelText("Range"), {
    target: { value: "custom" },
  });
  expect(onChange).toHaveBeenCalledWith({
    range: "custom",
    from: "2026-10-06",
    to: "2026-10-06",
  });

  const custom = {
    preset: "custom" as const,
    since: "2026-10-01",
    until: "2026-11-01",
  };
  render(
    <DateRangePicker
      value={custom}
      search={{ range: "custom", from: "2026-10-01", to: "2026-10-31" }}
      onChange={onChange}
    />,
  );
  fireEvent.change(screen.getByLabelText("To"), {
    target: { value: "2026-10-15" },
  });
  expect(onChange).toHaveBeenLastCalledWith({
    range: "custom",
    from: "2026-10-01",
    to: "2026-10-15",
  });
});

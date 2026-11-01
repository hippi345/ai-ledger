import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { LiveIndicator } from "./LiveIndicator";

test("shows live state and toggles", () => {
  const onToggle = vi.fn();
  const { rerender } = render(
    <LiveIndicator paused={false} lastIngest={null} onToggle={onToggle} />,
  );
  expect(screen.getByRole("status")).toHaveTextContent("Live");
  expect(screen.getByRole("status")).not.toHaveTextContent("updated");
  fireEvent.click(screen.getByRole("button", { name: "Pause" }));
  expect(onToggle).toHaveBeenCalled();
  rerender(
    <LiveIndicator
      paused
      lastIngest={new Date(2026, 10, 1, 9, 5)}
      onToggle={onToggle}
    />,
  );
  expect(screen.getByRole("status")).toHaveTextContent("Paused");
  expect(screen.getByRole("status")).toHaveTextContent("updated");
  expect(screen.getByRole("button", { name: "Resume" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});

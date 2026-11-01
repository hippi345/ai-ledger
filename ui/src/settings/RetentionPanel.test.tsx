import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { RetentionPanel } from "./RetentionPanel";

test("loads, sets and resets retention", async () => {
  invoke.mockImplementation(async (cmd: string) =>
    cmd === "get_setting" ? 90 : cmd === "reset_setting" ? true : null,
  );
  render(<RetentionPanel />);
  const select = (await screen.findByLabelText(
    "Keep raw events",
  )) as HTMLSelectElement;
  await waitFor(() => expect(select.value).toBe("90"));
  fireEvent.change(select, { target: { value: "30" } });
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("set_setting", {
      key: "retention.raw_event_days",
      value: 30,
    }),
  );
  fireEvent.change(select, { target: { value: "0" } });
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("reset_setting", {
      key: "retention.raw_event_days",
    }),
  );
  await waitFor(() => expect(select.value).toBe("0"));
});

test("unset means keep forever", async () => {
  invoke.mockResolvedValue(null);
  render(<RetentionPanel />);
  const select = (await screen.findByLabelText(
    "Keep raw events",
  )) as HTMLSelectElement;
  await waitFor(() => expect(select).toBeEnabled());
  expect(select.value).toBe("0");
});

import { render, screen } from "@testing-library/react";
import { vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));

import { Summary } from "./Summary";

test("loads cards from the backend", async () => {
  invoke.mockImplementation(async (cmd: string) =>
    cmd === "summary_cards" ? [{ label: "Today", text: "Today: 900 tokens, ~$0.02." }] : undefined,
  );
  render(<Summary />);
  expect(await screen.findByText("Today: 900 tokens, ~$0.02.")).toBeInTheDocument();
});

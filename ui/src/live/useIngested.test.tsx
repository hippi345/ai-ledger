import { render, waitFor } from "@testing-library/react";
import { vi } from "vitest";

type Handler = (e: { payload: unknown }) => void;
const handlers = new Map<string, Handler>();
const unlisten = vi.fn();
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, h: Handler) => {
    handlers.set(name, h);
    return unlisten;
  },
}));

import { INGESTED_EVENT, useIngested } from "./useIngested";

function Probe({ onIngested }: { onIngested: (e: unknown) => void }) {
  useIngested(onIngested);
  return null;
}

test("delivers ingested events and unsubscribes on unmount", async () => {
  const seen = vi.fn();
  const { unmount } = render(<Probe onIngested={seen} />);
  await waitFor(() => expect(handlers.has(INGESTED_EVENT)).toBe(true));
  const payload = { sourceId: 1, inserted: 2, updated: 0 };
  handlers.get(INGESTED_EVENT)!({ payload });
  expect(seen).toHaveBeenCalledWith(payload);
  unmount();
  expect(unlisten).toHaveBeenCalled();
});

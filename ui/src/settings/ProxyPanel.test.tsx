import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));

import { ProxyPanel } from "./ProxyPanel";

const off = { enabled: false, port: 8788, running: false, error: null };

beforeEach(() => invoke.mockReset());

test("is off by default and shows URLs only once running", async () => {
  invoke.mockImplementation(async (cmd: string, a?: { port: number }) =>
    cmd === "proxy_status"
      ? off
      : { enabled: true, port: a?.port, running: true, error: null },
  );
  render(<ProxyPanel />);
  const box = await screen.findByRole("checkbox", {
    name: "Run the logging proxy",
  });
  await waitFor(() => expect(box).toBeEnabled());
  expect(box).not.toBeChecked();
  expect(screen.queryByRole("list", { name: "Proxy base URLs" })).toBeNull();
  fireEvent.change(screen.getByRole("textbox", { name: "Port" }), {
    target: { value: "9000" },
  });
  fireEvent.click(box);
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("set_proxy", {
      enabled: true,
      port: 9000,
    }),
  );
  expect(
    await screen.findByText("http://127.0.0.1:9000/anthropic"),
  ).toBeInTheDocument();
});

test("rejects a bad port without calling the backend", async () => {
  invoke.mockResolvedValue(off);
  render(<ProxyPanel />);
  const box = await screen.findByRole("checkbox", {
    name: "Run the logging proxy",
  });
  await waitFor(() => expect(box).toBeEnabled());
  fireEvent.change(screen.getByRole("textbox", { name: "Port" }), {
    target: { value: "80" },
  });
  fireEvent.click(box);
  expect(await screen.findByRole("alert")).toHaveTextContent("1024");
  expect(invoke).not.toHaveBeenCalledWith("set_proxy", expect.anything());
});

test("shows a start error from the backend", async () => {
  invoke.mockImplementation(async (cmd: string) =>
    cmd === "proxy_status"
      ? off
      : {
          enabled: true,
          port: 8788,
          running: false,
          error: "could not listen on 127.0.0.1:8788: in use",
        },
  );
  render(<ProxyPanel />);
  const box = await screen.findByRole("checkbox", {
    name: "Run the logging proxy",
  });
  await waitFor(() => expect(box).toBeEnabled());
  fireEvent.click(box);
  expect(await screen.findByRole("alert")).toHaveTextContent("in use");
});

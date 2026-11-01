import { useState } from "react";
import { RouterProvider, type RouterHistory } from "@tanstack/react-router";
import { makeRouter } from "./router";

/** The app: sidebar shell plus one route per page. */
export function App({ history }: { history?: RouterHistory }) {
  const [router] = useState(() => makeRouter(history));
  return <RouterProvider router={router} />;
}

import { useState } from "react";
import { SessionTimeline } from "./SessionTimeline";
import { SessionsPage } from "./SessionsPage";

/** Sessions route body: the list, or one session's timeline. */
export function Sessions() {
  const [open, setOpen] = useState<string | null>(null);
  return open ? (
    <SessionTimeline id={open} onBack={() => setOpen(null)} />
  ) : (
    <SessionsPage onOpen={setOpen} />
  );
}

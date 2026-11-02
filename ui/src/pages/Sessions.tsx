import { useState } from "react";
import { SessionTimeline } from "./SessionTimeline";
import { SessionsPage } from "./SessionsPage";

/** Sessions route body: the list, with the open session's timeline beside it. */
export function Sessions() {
  const [open, setOpen] = useState<string | null>(null);
  return (
    <div className={open ? "sessions two-pane" : "sessions"}>
      <div className="sessions-list">
        <SessionsPage onOpen={setOpen} selected={open} />
      </div>
      {open && (
        <div className="sessions-detail">
          <SessionTimeline id={open} onBack={() => setOpen(null)} />
        </div>
      )}
    </div>
  );
}

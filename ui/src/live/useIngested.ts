import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { IngestedEvent } from "../bindings";

/** Backend event sent after new usage is stored (see src-tauri/src/ipc.rs). */
export const INGESTED_EVENT = "ledger://ingested";

/** Calls `onIngested` each time the backend stores new or changed usage. */
export function useIngested(onIngested: (e: IngestedEvent) => void) {
  const cb = useRef(onIngested);
  cb.current = onIngested;
  useEffect(() => {
    let stop: (() => void) | undefined;
    let cancelled = false;
    void listen<IngestedEvent>(INGESTED_EVENT, (e) =>
      cb.current(e.payload),
    ).then((un) => {
      if (cancelled) un();
      else stop = un;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);
}

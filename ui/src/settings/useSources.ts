import { useCallback, useEffect, useState } from "react";
import { commands, type Candidate, type SourceInfo_Serialize as SourceInfo } from "../bindings";

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: { code: string; message: string } };

/** Loads sources from the backend and exposes the panel's actions. */
export function useSources() {
  const [sources, setSources] = useState<SourceInfo[]>([]);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(async <T,>(p: Promise<Result<T>>): Promise<T | undefined> => {
    const r = await p;
    if (r.status === "error") {
      setError(r.error.message);
      return undefined;
    }
    setError(null);
    return r.data;
  }, []);

  const refresh = useCallback(async () => {
    const list = await run(commands.listSources());
    if (list) setSources(list);
  }, [run]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const wrap = (fn: () => Promise<void>) => async () => {
    setBusy(true);
    try {
      await fn();
    } finally {
      setBusy(false);
    }
  };

  const detect = wrap(async () => {
    const found = await run(commands.detectSources());
    if (found) setCandidates(found);
  });

  const add = (c: Candidate) =>
    wrap(async () => {
      if ((await run(commands.addSource(c))) !== undefined) {
        setCandidates((cs) => cs.map((x) => (x === c ? { ...x, added: true } : x)));
        await refresh();
      }
    })();

  const toggle = (id: number, enabled: boolean) =>
    wrap(async () => {
      if ((await run(commands.setSourceEnabled(id, enabled))) !== undefined) await refresh();
    })();

  return { sources, candidates, busy, error, detect, add, toggle, refresh };
}

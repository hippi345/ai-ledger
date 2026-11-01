import { useCallback, useEffect, useState } from "react";
import { commands, type PriceOverride } from "../bindings";

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: { code: string; message: string } };

/** Loads price overrides and exposes save/remove (each re-prices all events). */
export function usePriceOverrides() {
  const [overrides, setOverrides] = useState<PriceOverride[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

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
    const list = await run(commands.listPriceOverrides());
    if (list) setOverrides(list);
  }, [run]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const change = async (p: Promise<Result<number>>) => {
    setBusy(true);
    try {
      const n = await run(p);
      if (n !== undefined) {
        setNotice(`Re-priced ${n} event${n === 1 ? "" : "s"}.`);
        await refresh();
      }
    } finally {
      setBusy(false);
    }
  };

  return {
    overrides,
    busy,
    error,
    notice,
    save: (p: PriceOverride) => change(commands.setPriceOverride(p)),
    remove: (model: string, from: string) => change(commands.removePriceOverride(model, from)),
  };
}

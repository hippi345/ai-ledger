import { useCallback, useEffect, useState } from "react";
import { commands, type SummaryCard } from "../bindings";
import { useIngested } from "../live/useIngested";

/** Loads the summary cards and reloads them whenever new usage is ingested. */
export function useSummaryCards() {
  const [cards, setCards] = useState<SummaryCard[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const r = await commands.summaryCards();
    if (r.status === "ok") {
      setCards(r.data);
      setError(null);
    } else {
      setError(r.error.message);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);
  useIngested(() => void refresh());

  return { cards, loading, error };
}

import { SummaryCards } from "./SummaryCards";
import { useSummaryCards } from "./useSummaryCards";

/** Summary cards connected to the backend, refreshed on ingest. */
export function Summary() {
  const s = useSummaryCards();
  return (
    <>
      {s.error && <p role="alert">{s.error}</p>}
      <SummaryCards cards={s.cards} loading={s.loading} />
    </>
  );
}

import { PricesPanel } from "./PricesPanel";
import { usePriceOverrides } from "./usePriceOverrides";

/** The price overrides panel connected to the backend. */
export function PricesSettings() {
  const s = usePriceOverrides();
  return (
    <>
      {s.error && <p role="alert">{s.error}</p>}
      {s.notice && <p role="status">{s.notice}</p>}
      <PricesPanel
        overrides={s.overrides}
        busy={s.busy}
        onSave={(p) => void s.save(p)}
        onRemove={(m, f) => void s.remove(m, f)}
      />
    </>
  );
}

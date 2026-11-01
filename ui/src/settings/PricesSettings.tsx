import { useState } from "react";
import { PricesPanel } from "./PricesPanel";
import { UnknownModelsPanel } from "./UnknownModelsPanel";
import { usePriceOverrides } from "./usePriceOverrides";

/** The price overrides and unpriced models panels connected to the backend. */
export function PricesSettings() {
  const s = usePriceOverrides();
  const [prefill, setPrefill] = useState<string | undefined>();
  return (
    <>
      {s.error && <p role="alert">{s.error}</p>}
      {s.notice && <p role="status">{s.notice}</p>}
      <PricesPanel
        overrides={s.overrides}
        busy={s.busy}
        prefillModel={prefill}
        onSave={(p) => void s.save(p)}
        onRemove={(m, f) => void s.remove(m, f)}
      />
      <UnknownModelsPanel models={s.unknown} onAddPrice={setPrefill} />
    </>
  );
}

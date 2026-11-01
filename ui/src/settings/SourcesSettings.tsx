import { ImportPanel } from "./ImportPanel";
import { SourcesPanel } from "./SourcesPanel";
import { useSources } from "./useSources";

/** The Sources panel connected to the backend. */
export function SourcesSettings() {
  const s = useSources();
  return (
    <>
      {s.error && <p role="alert">{s.error}</p>}
      <SourcesPanel
        sources={s.sources}
        candidates={s.candidates}
        busy={s.busy}
        onDetect={() => void s.detect()}
        onAdd={(c) => void s.add(c)}
        onToggle={(id, on) => void s.toggle(id, on)}
      />
      <ImportPanel onAdded={() => void s.refresh()} />
    </>
  );
}

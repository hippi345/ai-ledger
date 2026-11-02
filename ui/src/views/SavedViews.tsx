import { useEffect, useId, useRef, useState } from "react";
import type { SavedView } from "./views";

type Props = {
  views: SavedView[];
  onSave: (name: string) => void;
  onApply: (view: SavedView) => void;
  onDelete: (name: string) => void;
};

/** Save the current range and filters under a name; apply or delete saved ones. */
export function SavedViews({ views, onSave, onApply, onDelete }: Props) {
  const [name, setName] = useState("");
  const [open, setOpen] = useState(false);
  const panelId = useId();
  const root = useRef<HTMLDivElement>(null);
  // Close the popover on Escape or a click outside it.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    const onDown = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("mousedown", onDown);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onDown);
    };
  }, [open]);
  return (
    <div role="group" aria-label="Saved views" ref={root}>
      <button
        type="button"
        className="views-toggle"
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => setOpen((o) => !o)}
      >
        Views
      </button>
      <div id={panelId} className="popover" hidden={!open}>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (!name.trim()) return;
            onSave(name);
            setName("");
          }}
        >
          <input
            aria-label="View name"
            placeholder="Name this view"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <button type="submit" className="btn-primary" disabled={!name.trim()}>
            Save view
          </button>
        </form>
        {views.length > 0 && (
          <ul>
            {views.map((v) => (
              <li key={v.name}>
                <button
                  type="button"
                  onClick={() => {
                    onApply(v);
                    setOpen(false);
                  }}
                >
                  {v.name}
                </button>
                <button
                  type="button"
                  aria-label={`Delete view ${v.name}`}
                  onClick={() => onDelete(v.name)}
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

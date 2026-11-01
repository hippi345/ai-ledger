import { useState } from "react";
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
  return (
    <div role="group" aria-label="Saved views">
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
              <button type="button" onClick={() => onApply(v)}>
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
  );
}

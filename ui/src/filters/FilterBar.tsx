import {
  FILTER_LABELS,
  activeChips,
  type FilterDim,
  type FilterSearch,
} from "./filters";

type Props = {
  filters: FilterSearch;
  onRemove: (dim: FilterDim, value: string) => void;
  onClear: () => void;
};

/** Active filter chips; each can be removed, or all cleared. Hidden when empty. */
export function FilterBar({ filters, onRemove, onClear }: Props) {
  const chips = activeChips(filters);
  if (chips.length === 0) return null;
  return (
    <div role="toolbar" aria-label="Filters">
      {chips.map(([dim, value]) => (
        <button
          key={`${dim}:${value}`}
          type="button"
          aria-label={`Remove ${FILTER_LABELS[dim]} ${value || "Unknown"}`}
          onClick={() => onRemove(dim, value)}
        >
          {FILTER_LABELS[dim]}: {value || "Unknown"} ×
        </button>
      ))}
      <button type="button" onClick={onClear}>
        Clear filters
      </button>
    </div>
  );
}

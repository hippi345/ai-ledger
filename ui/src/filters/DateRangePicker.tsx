import {
  PRESETS,
  PRESET_LABELS,
  type DateRange,
  type Preset,
  type RangeSearch,
} from "./range";

type Props = {
  value: DateRange;
  search: RangeSearch;
  onChange: (next: RangeSearch) => void;
};

/** Preset select plus custom from/to dates (to is inclusive). */
export function DateRangePicker({ value, search, onChange }: Props) {
  const custom = value.preset === "custom";
  return (
    <fieldset aria-label="Date range">
      <label>
        Range{" "}
        <select
          value={value.preset}
          onChange={(e) => {
            const range = e.target.value as Preset;
            onChange(
              range === "custom"
                ? { range, from: value.since, to: search.to ?? value.since }
                : { range },
            );
          }}
        >
          {PRESETS.map((p) => (
            <option key={p} value={p}>
              {PRESET_LABELS[p]}
            </option>
          ))}
        </select>
      </label>
      {custom && (
        <>
          <label>
            From{" "}
            <input
              type="date"
              value={search.from ?? ""}
              onChange={(e) => onChange({ ...search, from: e.target.value })}
            />
          </label>
          <label>
            To{" "}
            <input
              type="date"
              value={search.to ?? ""}
              onChange={(e) => onChange({ ...search, to: e.target.value })}
            />
          </label>
        </>
      )}
    </fieldset>
  );
}

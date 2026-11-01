import { useState, type FormEvent } from "react";
import type { PriceOverride } from "../bindings";

type Props = {
  overrides: PriceOverride[];
  busy: boolean;
  onSave: (p: PriceOverride) => void;
  onRemove: (model: string, effectiveFrom: string) => void;
};

const RATES = [
  ["input", "Input"],
  ["output", "Output"],
  ["cacheRead", "Cache read"],
  ["cacheWrite", "Cache write"],
] as const;

const empty = { model: "", effectiveFrom: "", input: "", output: "", cacheRead: "0", cacheWrite: "0" };

/** Lists user price overrides (USD per million tokens) and adds new ones. */
export function PricesPanel({ overrides, busy, onSave, onRemove }: Props) {
  const [form, setForm] = useState(empty);
  const set = (k: keyof typeof empty) => (e: { target: { value: string } }) =>
    setForm((f) => ({ ...f, [k]: e.target.value }));
  const submit = (e: FormEvent) => {
    e.preventDefault();
    onSave({
      model: form.model.trim(),
      effectiveFrom: form.effectiveFrom,
      input: Number(form.input),
      output: Number(form.output),
      cacheRead: Number(form.cacheRead),
      cacheWrite: Number(form.cacheWrite),
    });
    setForm(empty);
  };
  return (
    <section aria-labelledby="prices-h">
      <h2 id="prices-h">Price overrides</h2>
      <p>Your own rates, in USD per million tokens. They replace bundled prices from the date you pick.</p>
      {overrides.length === 0 ? (
        <p>No overrides. Bundled estimates apply.</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Model</th>
              <th>From</th>
              {RATES.map(([, l]) => (
                <th key={l}>{l}</th>
              ))}
              <th />
            </tr>
          </thead>
          <tbody>
            {overrides.map((o) => (
              <tr key={`${o.model}@${o.effectiveFrom}`}>
                <td>{o.model}</td>
                <td>{o.effectiveFrom}</td>
                {RATES.map(([k]) => (
                  <td key={k}>{o[k]}</td>
                ))}
                <td>
                  <button
                    type="button"
                    disabled={busy}
                    aria-label={`Remove ${o.model} from ${o.effectiveFrom}`}
                    onClick={() => onRemove(o.model, o.effectiveFrom)}
                  >
                    Remove
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <form onSubmit={submit} aria-label="Add price override">
        <label>
          Model <input required value={form.model} onChange={set("model")} />
        </label>
        <label>
          From <input required type="date" value={form.effectiveFrom} onChange={set("effectiveFrom")} />
        </label>
        {RATES.map(([k, l]) => (
          <label key={k}>
            {l}{" "}
            <input required type="number" min="0" step="any" value={form[k]} onChange={set(k)} />
          </label>
        ))}
        <button type="submit" disabled={busy}>
          Save override
        </button>
      </form>
    </section>
  );
}

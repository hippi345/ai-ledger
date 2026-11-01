import type { UnknownModelInfo } from "../bindings";

type Props = {
  models: UnknownModelInfo[];
  onAddPrice: (model: string) => void;
};

/** Models seen in usage that have no price, so their cost shows as unknown. */
export function UnknownModelsPanel({ models, onAddPrice }: Props) {
  return (
    <section aria-label="Unpriced models">
      <h3>Unpriced models</h3>
      {models.length === 0 ? (
        <p>Every model in your usage has a price.</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Model</th>
              <th>Events</th>
              <th>Tokens</th>
              <th>Last seen</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {models.map((m) => (
              <tr key={m.model}>
                <td>{m.model}</td>
                <td>{m.events.toLocaleString()}</td>
                <td>{m.tokens.toLocaleString()}</td>
                <td>{m.lastSeen.slice(0, 10)}</td>
                <td>
                  <button type="button" onClick={() => onAddPrice(m.model)} aria-label={`Add price for ${m.model}`}>
                    Add price
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

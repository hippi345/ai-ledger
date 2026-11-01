import type { SummaryCard } from "../bindings";

type Props = { cards: SummaryCard[]; loading?: boolean };

/** Plain-English summary cards: today, this week, this month. */
export function SummaryCards({ cards, loading = false }: Props) {
  if (loading && cards.length === 0) return <p aria-busy="true">Loading summary…</p>;
  return (
    <section aria-label="Summary">
      {cards.map((c) => (
        <article key={c.label} aria-label={c.label}>
          <h3>{c.label}</h3>
          <p>{c.text}</p>
        </article>
      ))}
    </section>
  );
}

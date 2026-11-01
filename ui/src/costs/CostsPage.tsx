import { Chart } from "../charts/Chart";
import { ExportCsvButton } from "../export/ExportCsvButton";
import { useWidgetData } from "../overview/widgetData";
import { BudgetPanel } from "./BudgetPanel";
import { Prices } from "./PriceTable";
import { useBudget } from "./useBudget";

/** Costs page: budget and alerts, cost trend with projection, and the price table. */
export function CostsPage() {
  const b = useBudget();
  const trend = useWidgetData("cost-trend");
  return (
    <>
      {b.error && <p role="alert">{b.error}</p>}
      <BudgetPanel
        budget={b.budget}
        spent={b.spent}
        projected={b.projected}
        onSave={(v) => void b.save(v)}
      />
      <section aria-labelledby="trend-h">
        <h3 id="trend-h">Cost trend</h3>
        {trend.error && <p role="alert">{trend.error}</p>}
        {trend.data && (
          <>
            <ExportCsvButton
              name="Cost trend"
              header={trend.data.header}
              rows={trend.data.rows}
            />
            <Chart
              label="Cost per day with projection"
              option={trend.data.option}
              exportName="Cost trend"
            />
          </>
        )}
      </section>
      <section aria-labelledby="prices-h">
        <h3 id="prices-h">Prices</h3>
        <Prices />
      </section>
    </>
  );
}

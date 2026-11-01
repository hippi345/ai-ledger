import type { EChartsCoreOption } from "echarts/core";
import { PALETTE, formatCost } from "./theme";

/** Daily cost points, `day` as UTC `YYYY-MM-DD`. */
export type CostPoint = { day: string; cost: number };

const DAY = 86_400_000;
const parse = (d: string) => Date.parse(`${d}T00:00:00Z`);
const fmtDay = (t: number) => new Date(t).toISOString().slice(0, 10);

/**
 * Projected total by `periodEnd` (exclusive): spend so far plus the average
 * daily spend over the days seen so far, times the days left.
 */
export function projectTotal(points: CostPoint[], periodStart: string, periodEnd: string, today: string) {
  const spent = points.reduce((s, p) => s + p.cost, 0);
  const elapsed = Math.max(1, Math.round((parse(today) - parse(periodStart)) / DAY) + 1);
  const left = Math.max(0, Math.round((parse(periodEnd) - parse(today)) / DAY) - 1);
  return spent + (spent / elapsed) * left;
}

/** Cumulative cost line with a dashed projection to the end of the period (spec §C). */
export function costTrendOption(
  points: CostPoint[],
  periodStart: string,
  periodEnd: string,
  today: string,
): EChartsCoreOption {
  const byDay = new Map(points.map((p) => [p.day, p.cost]));
  const days: string[] = [];
  for (let t = parse(periodStart); t < parse(periodEnd); t += DAY) days.push(fmtDay(t));
  let run = 0;
  const actual = days.map((d) => {
    if (d > today) return null;
    run += byDay.get(d) ?? 0;
    return run;
  });
  const projected = projectTotal(points, periodStart, periodEnd, today);
  const iToday = days.indexOf(today);
  const lastIdx = days.length - 1;
  const projection = days.map((_, i) => {
    if (iToday < 0 || i < iToday) return null;
    if (lastIdx === iToday) return run;
    return run + ((projected - run) * (i - iToday)) / (lastIdx - iToday);
  });
  return {
    tooltip: { trigger: "axis", valueFormatter: (v: number) => formatCost(v) },
    legend: { bottom: 0 },
    xAxis: { type: "category", data: days, boundaryGap: false },
    yAxis: { type: "value", axisLabel: { formatter: (v: number) => formatCost(v) } },
    series: [
      { name: "Cost so far", type: "line", showSymbol: false, color: PALETTE[0], data: actual },
      {
        name: "Projected",
        type: "line",
        showSymbol: false,
        color: PALETTE[0],
        lineStyle: { type: "dashed" },
        data: projection,
      },
    ],
  };
}

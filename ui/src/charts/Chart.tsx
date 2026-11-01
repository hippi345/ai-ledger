import { useEffect, useRef } from "react";
import * as echarts from "echarts/core";
import { BarChart, HeatmapChart, LineChart, PieChart } from "echarts/charts";
import {
  GridComponent,
  LegendComponent,
  TooltipComponent,
  VisualMapComponent,
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { EChartsCoreOption } from "echarts/core";
import { baseOption } from "./theme";

echarts.use([
  BarChart,
  HeatmapChart,
  LineChart,
  PieChart,
  GridComponent,
  LegendComponent,
  TooltipComponent,
  VisualMapComponent,
  CanvasRenderer,
]);

type Props = {
  option: EChartsCoreOption;
  /** Accessible description of what the chart shows. */
  label: string;
  height?: number;
  onClick?: (params: { name: string; seriesName?: string }) => void;
};

/** One ECharts instance bound to a div; resizes with its container. */
export function Chart({ option, label, height = 280, onClick }: Props) {
  const el = useRef<HTMLDivElement>(null);
  const chart = useRef<ReturnType<typeof echarts.init> | null>(null);

  useEffect(() => {
    if (!el.current) return;
    const c = echarts.init(el.current);
    chart.current = c;
    const ro =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(() => c.resize());
    ro?.observe(el.current);
    return () => {
      ro?.disconnect();
      c.dispose();
      chart.current = null;
    };
  }, []);

  useEffect(() => {
    chart.current?.setOption({ ...baseOption, ...option }, true);
  }, [option]);

  useEffect(() => {
    const c = chart.current;
    if (!c || !onClick) return;
    const h = (p: unknown) =>
      onClick(p as { name: string; seriesName?: string });
    c.on("click", h);
    return () => {
      c.off("click", h);
    };
  }, [onClick]);

  return (
    <div
      ref={el}
      role="img"
      aria-label={label}
      style={{ width: "100%", height }}
    />
  );
}

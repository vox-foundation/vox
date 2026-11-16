import React from 'react';
import {
  ResponsiveContainer,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
} from 'recharts';
import {
  AXIS_TICK_STYLE,
  BRASS_STROKE,
  GRID_STROKE,
  OptionalBrush,
  OptionalReferenceLine,
  ChartWidgetEmpty,
  ChartWidgetFrame,
  formatChartXTick,
  formatCompactMetricValue,
  tooltipStyle,
  useChartPointSelection,
  type ChartWidgetProps,
} from './chartWidgetShared';

export function AreaChartWidget({
  title,
  series,
  referenceCap,
  referenceLabel,
  unit,
  valueFormatter,
  enableBrush,
  onPointSelect,
}: ChartWidgetProps) {
  const { selectedText, handleChartClick, clearSelection } =
    useChartPointSelection(series, onPointSelect, unit, valueFormatter);

  if (series.length === 0) {
    return <ChartWidgetEmpty title={title} />;
  }

  const showBrush = enableBrush ?? series.length >= 6;
  const showReferenceCap =
    typeof referenceCap === 'number' && Number.isFinite(referenceCap);

  return (
    <ChartWidgetFrame
      title={title}
      selectedText={selectedText}
      onClearSelection={clearSelection}
    >
      <ResponsiveContainer width="100%" height="100%" minHeight={156}>
        <AreaChart
          data={series}
          margin={{ top: 6, right: 8, left: 0, bottom: 2 }}
          onClick={handleChartClick}
        >
          <CartesianGrid stroke={GRID_STROKE} strokeDasharray="3 3" vertical={false} />
          <XAxis
            dataKey="t"
            tickFormatter={formatChartXTick}
            tick={AXIS_TICK_STYLE}
            tickLine={false}
            axisLine={false}
            minTickGap={16}
          />
          <YAxis
            width={36}
            domain={['auto', 'auto']}
            tickFormatter={(v) =>
              formatCompactMetricValue(Number(v), unit, valueFormatter)
            }
            tick={AXIS_TICK_STYLE}
            tickLine={false}
            axisLine={false}
          />
          <Tooltip
            {...tooltipStyle}
            labelFormatter={(label) => formatChartXTick(label)}
            formatter={(value) => [
              formatCompactMetricValue(Number(value), unit, valueFormatter),
              title,
            ]}
          />
          {showReferenceCap && OptionalReferenceLine ? (
            <OptionalReferenceLine
              y={referenceCap}
              stroke="var(--color-accent-default)"
              strokeDasharray="4 3"
              label={
                referenceLabel
                  ? {
                      value: referenceLabel,
                      fill: 'var(--color-text-muted)',
                      fontSize: 10,
                      position: 'insideTopRight',
                    }
                  : undefined
              }
            />
          ) : null}
          <Area
            type="monotone"
            dataKey="v"
            stroke={BRASS_STROKE}
            fill={BRASS_STROKE}
            fillOpacity={0.2}
            strokeWidth={2}
            dot={false}
            activeDot={{ r: 4, fill: BRASS_STROKE }}
          />
          {showBrush && OptionalBrush ? (
            <OptionalBrush
              dataKey="t"
              height={18}
              stroke="var(--color-border-strong)"
              fill="var(--color-bg-elevated)"
              travellerWidth={7}
              tickFormatter={formatChartXTick}
            />
          ) : null}
        </AreaChart>
      </ResponsiveContainer>
    </ChartWidgetFrame>
  );
}

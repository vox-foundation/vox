import React from 'react';
import * as RechartsNamespace from 'recharts';
import { Glass } from '../../ui/Glass';
import type { MetricPoint } from '../../../hooks/useMetricSeries';

export const BRASS_STROKE = 'rgb(var(--brass))';
export const GRID_STROKE = 'var(--color-border-subtle)';
export const AXIS_TICK_STYLE = {
  fontSize: 10,
  fill: 'var(--color-text-muted)',
} as const;

export const OptionalBrush =
  'Brush' in RechartsNamespace ? RechartsNamespace.Brush : null;
export const OptionalReferenceLine =
  'ReferenceLine' in RechartsNamespace ? RechartsNamespace.ReferenceLine : null;

export interface ChartWidgetProps {
  title: string;
  series: MetricPoint[];
  referenceCap?: number;
  referenceLabel?: string;
  unit?: string;
  valueFormatter?: (v: number) => string;
  enableBrush?: boolean;
  onPointSelect?: (point: { t: number; v: number }) => void;
}

export function formatChartXTick(value: unknown): string {
  const num = typeof value === 'number' ? value : Number(value);
  if (!Number.isFinite(num)) {
    return String(value ?? '');
  }
  if (num > 1e11) {
    const date = new Date(num);
    const hh = String(date.getHours()).padStart(2, '0');
    const mm = String(date.getMinutes()).padStart(2, '0');
    const ss = String(date.getSeconds()).padStart(2, '0');
    return `${hh}:${mm}:${ss}`;
  }
  return String(Math.round(num));
}

export function formatCompactMetricValue(
  value: number,
  unit?: string,
  valueFormatter?: (v: number) => string,
): string {
  if (valueFormatter) {
    return valueFormatter(value);
  }
  if (!Number.isFinite(value)) {
    return '0';
  }
  const abs = Math.abs(value);
  let formatted: string;
  if (abs >= 1_000_000) {
    formatted = `${(value / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`;
  } else if (abs >= 1_000) {
    formatted = `${(value / 1_000).toFixed(1).replace(/\.0$/, '')}k`;
  } else if (Number.isInteger(value)) {
    formatted = String(value);
  } else {
    formatted = value.toFixed(2).replace(/\.?0+$/, '');
  }
  if (!unit) {
    return formatted;
  }
  return unit === '$' ? `$${formatted}` : `${formatted}${unit}`;
}

export function resolveClickedPoint(
  series: MetricPoint[],
  state: unknown,
): MetricPoint | null {
  if (!state || typeof state !== 'object' || series.length === 0) {
    return null;
  }
  const record = state as Record<string, unknown>;

  if (typeof record.t === 'number' && typeof record.v === 'number') {
    return { t: record.t, v: record.v };
  }

  const directPayload = record.payload as Record<string, unknown> | undefined;
  if (
    directPayload &&
    typeof directPayload.t === 'number' &&
    typeof directPayload.v === 'number'
  ) {
    return { t: directPayload.t, v: directPayload.v };
  }

  const activePayload = record.activePayload;
  if (Array.isArray(activePayload) && activePayload.length > 0) {
    const first = activePayload[0] as Record<string, unknown> | undefined;
    const nested = first?.payload as Record<string, unknown> | undefined;
    if (
      nested &&
      typeof nested.t === 'number' &&
      typeof nested.v === 'number'
    ) {
      return { t: nested.t, v: nested.v };
    }
  }

  const rawIdx = record.activeTooltipIndex ?? record.activeIndex;
  if (typeof rawIdx === 'number' && Number.isInteger(rawIdx)) {
    const point = series[rawIdx];
    if (point) {
      return point;
    }
  } else if (typeof rawIdx === 'string' && rawIdx.trim() !== '') {
    const parsedIdx = Number(rawIdx);
    if (Number.isInteger(parsedIdx) && series[parsedIdx]) {
      return series[parsedIdx]!;
    }
  }

  const activeLabel = record.activeLabel;
  if (activeLabel !== undefined && activeLabel !== null) {
    const labelNum = Number(activeLabel);
    if (Number.isFinite(labelNum)) {
      const matched = series.find((entry) => entry.t === labelNum);
      if (matched) {
        return matched;
      }
    }
  }

  return null;
}

export function useChartPointSelection(
  series: MetricPoint[],
  onPointSelect?: (point: { t: number; v: number }) => void,
  unit?: string,
  valueFormatter?: (v: number) => string,
) {
  const [selectedPoint, setSelectedPoint] = React.useState<MetricPoint | null>(
    null,
  );

  const selectPoint = React.useCallback(
    (point: MetricPoint | null) => {
      setSelectedPoint(point);
      if (point) {
        onPointSelect?.({ t: point.t, v: point.v });
      }
    },
    [onPointSelect],
  );

  const handleChartClick = React.useCallback(
    (state: unknown) => {
      const point = resolveClickedPoint(series, state);
      if (point) {
        selectPoint(point);
      }
    },
    [series, selectPoint],
  );

  const clearSelection = React.useCallback(() => {
    setSelectedPoint(null);
  }, []);

  const selectedText = selectedPoint
    ? formatCompactMetricValue(selectedPoint.v, unit, valueFormatter)
    : null;

  return {
    selectedPoint,
    selectedText,
    selectPoint,
    handleChartClick,
    clearSelection,
  };
}

export function ChartWidgetEmpty({ title }: Pick<ChartWidgetProps, 'title'>) {
  return (
    <Glass className="flex h-full min-h-[200px] w-full min-w-0 flex-col p-4">
      <h3 className="font-display text-[14px] font-semibold tracking-wide text-text-primary">{title}</h3>
      <div
        role="status"
        className="mt-3 flex flex-1 items-center justify-center rounded-lg border border-dashed border-border-subtle px-4 py-8 text-center text-[11px] text-text-muted"
      >
        No data yet — waiting for metric samples.
      </div>
    </Glass>
  );
}

export function ChartWidgetFrame({
  title,
  selectedText,
  onClearSelection,
  children,
}: {
  title: string;
  selectedText?: string | null;
  onClearSelection?: () => void;
  children: React.ReactNode;
}) {
  return (
    <Glass className="flex h-full min-h-[200px] w-full min-w-0 flex-col px-3.5 py-2">
      <div className="flex min-w-0 items-center justify-between gap-2">
        <h3 className="truncate font-display text-[13px] font-semibold leading-tight tracking-wide text-text-primary">
          {title}
        </h3>
        {selectedText ? (
          <span
            data-testid="chart-selected-pill"
            className="inline-flex shrink-0 items-center gap-1.5 rounded-full border border-border-subtle bg-bg-elevated px-2 py-0.5 font-mono text-[10px] text-text-primary"
          >
            <span>Selected: {selectedText}</span>
            {onClearSelection && (
              <button
                type="button"
                onClick={onClearSelection}
                aria-label={`Clear selected ${title} datum`}
                className="rounded px-1 text-text-muted hover:text-text-primary"
              >
                Clear
              </button>
            )}
          </span>
        ) : null}
      </div>
      <div
        role="img"
        aria-label={`${title} chart`}
        className="relative mt-1 flex-1 min-h-[156px] w-full min-w-0"
      >
        {children}
      </div>
    </Glass>
  );
}

export const tooltipStyle = {
  contentStyle: {
    background: 'var(--color-bg-elevated)',
    border: '1px solid var(--color-border-subtle)',
    borderRadius: 8,
    fontSize: 11,
  },
  labelStyle: { color: 'var(--color-text-secondary)' },
  itemStyle: { color: BRASS_STROKE },
} as const;

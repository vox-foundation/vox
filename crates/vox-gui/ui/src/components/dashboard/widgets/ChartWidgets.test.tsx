import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import { LineChartWidget } from './LineChartWidget';
import { AreaChartWidget } from './AreaChartWidget';
import { BarChartWidget } from './BarChartWidget';
import { formatChartXTick, formatCompactMetricValue } from './chartWidgetShared';

const capturedProps: {
  responsive: Array<Record<string, unknown>>;
  xAxis: Array<Record<string, unknown>>;
  yAxis: Array<Record<string, unknown>>;
  brush: Array<Record<string, unknown>>;
  referenceLine: Array<Record<string, unknown>>;
} = {
  responsive: [],
  xAxis: [],
  yAxis: [],
  brush: [],
  referenceLine: [],
};

function resetCaptured() {
  capturedProps.responsive = [];
  capturedProps.xAxis = [];
  capturedProps.yAxis = [];
  capturedProps.brush = [];
  capturedProps.referenceLine = [];
}

vi.mock('recharts', () => ({
  ResponsiveContainer: ({
    children,
    ...rest
  }: {
    children: React.ReactNode;
    [key: string]: unknown;
  }) => {
    capturedProps.responsive.push(rest);
    return <div data-testid="responsive-container">{children}</div>;
  },
  LineChart: ({
    children,
    onClick,
  }: {
    children: React.ReactNode;
    onClick?: (state: unknown) => void;
  }) => (
    <svg
      data-testid="mock-line-chart"
      onClick={() => onClick?.({ activeTooltipIndex: 1, activeLabel: 1 })}
    >
      {children}
    </svg>
  ),
  AreaChart: ({
    children,
    onClick,
  }: {
    children: React.ReactNode;
    onClick?: (state: unknown) => void;
  }) => (
    <svg
      data-testid="mock-area-chart"
      onClick={() => onClick?.({ activeTooltipIndex: 2, activeLabel: 2 })}
    >
      {children}
    </svg>
  ),
  BarChart: ({
    children,
    onClick,
  }: {
    children: React.ReactNode;
    onClick?: (state: unknown) => void;
  }) => (
    <svg
      data-testid="mock-bar-chart"
      onClick={() => onClick?.({ activeTooltipIndex: 0, activeLabel: 0 })}
    >
      {children}
    </svg>
  ),
  Line: () => <g data-testid="mock-line" />,
  Area: () => <g data-testid="mock-area" />,
  Bar: ({ onClick }: { onClick?: (item: unknown, index: number) => void }) => (
    <rect
      data-testid="mock-bar"
      onClick={(e) => {
        e.stopPropagation();
        onClick?.({}, 1);
      }}
    />
  ),
  XAxis: (props: Record<string, unknown>) => {
    capturedProps.xAxis.push(props);
    return <g data-testid="mock-xaxis" />;
  },
  YAxis: (props: Record<string, unknown>) => {
    capturedProps.yAxis.push(props);
    return <g data-testid="mock-yaxis" />;
  },
  CartesianGrid: () => <g data-testid="mock-grid" />,
  Tooltip: () => null,
  Brush: (props: Record<string, unknown>) => {
    capturedProps.brush.push(props);
    return <g data-testid="mock-brush" />;
  },
  ReferenceLine: (props: Record<string, unknown>) => {
    capturedProps.referenceLine.push(props);
    return <g data-testid="mock-reference-line" />;
  },
}));

describe('ChartWidgets (434-D full-bleed scrubbing & drill-down)', () => {
  it('uses full-bleed responsive container and min-height stage across Line, Area, and Bar charts', () => {
    resetCaptured();
    const series = [
      { t: 0, v: 4 },
      { t: 1, v: 8 },
    ];
    render(
      <div>
        <LineChartWidget title="Latency" series={series} />
        <AreaChartWidget title="Spend" series={series} />
        <BarChartWidget title="Queue" series={series} />
      </div>,
    );

    expect(capturedProps.responsive).toHaveLength(3);
    for (const rc of capturedProps.responsive) {
      expect(rc.width).toBe('100%');
      expect(rc.height).toBe('100%');
      expect(rc.minHeight).toBe(156);
    }

    const stage = screen.getByRole('img', { name: 'Latency chart' });
    expect(stage.className).toContain('flex-1');
    expect(stage.className).toContain('min-h-[156px]');

    expect(capturedProps.yAxis).toHaveLength(3);
    for (const yAxis of capturedProps.yAxis) {
      expect(yAxis.width).toBe(36);
      expect(yAxis.tickLine).toBe(false);
      expect(yAxis.axisLine).toBe(false);
    }
    for (const xAxis of capturedProps.xAxis) {
      expect(xAxis.dataKey).toBe('t');
      expect(xAxis.tickLine).toBe(false);
      expect(xAxis.axisLine).toBe(false);
    }
  });

  it('auto-enables Brush when series length >= 6 and renders ReferenceLine when referenceCap is provided', () => {
    resetCaptured();
    const sixPoints = Array.from({ length: 6 }, (_, i) => ({ t: i, v: i * 2.5 }));
    const { rerender } = render(
      <AreaChartWidget
        title="Budget Burn"
        series={sixPoints}
        referenceCap={10}
        referenceLabel="Budget cap"
        unit="$"
      />,
    );

    expect(screen.getByTestId('mock-brush')).toBeInTheDocument();
    expect(capturedProps.brush[0]).toMatchObject({
      dataKey: 't',
      height: 18,
      stroke: 'var(--color-border-strong)',
      fill: 'var(--color-bg-elevated)',
      travellerWidth: 7,
    });
    expect(screen.getByTestId('mock-reference-line')).toBeInTheDocument();
    expect(capturedProps.referenceLine[0]).toMatchObject({
      y: 10,
      stroke: 'var(--color-accent-default)',
      strokeDasharray: '4 3',
    });

    resetCaptured();
    rerender(
      <AreaChartWidget
        title="Budget Burn"
        series={sixPoints.slice(0, 3)}
      />,
    );
    expect(screen.queryByTestId('mock-brush')).toBeNull();
    expect(screen.queryByTestId('mock-reference-line')).toBeNull();
  });

  it('selects datum on click, invokes onPointSelect, shows formatted pill, and clears on 1-click Clear', async () => {
    resetCaptured();
    const user = userEvent.setup();
    const onPointSelect = vi.fn();
    const series = [
      { t: 0, v: 1.5 },
      { t: 1, v: 4.25 },
      { t: 2, v: 9 },
    ];

    render(
      <LineChartWidget
        title="Budget Burn"
        series={series}
        unit="$"
        onPointSelect={onPointSelect}
      />,
    );

    expect(screen.queryByTestId('chart-selected-pill')).toBeNull();

    await user.click(screen.getByTestId('mock-line-chart'));
    expect(onPointSelect).toHaveBeenCalledWith({ t: 1, v: 4.25 });
    expect(screen.getByTestId('chart-selected-pill')).toHaveTextContent('Selected: $4.25');

    await user.click(
      screen.getByRole('button', { name: /clear selected budget burn datum/i }),
    );
    expect(screen.queryByTestId('chart-selected-pill')).toBeNull();
  });

  it('supports direct bar click datum selection in BarChartWidget', async () => {
    const user = userEvent.setup();
    const onPointSelect = vi.fn();
    render(
      <BarChartWidget
        title="Queue Depth"
        series={[
          { t: 0, v: 2 },
          { t: 1, v: 7 },
        ]}
        onPointSelect={onPointSelect}
      />,
    );

    await user.click(screen.getByTestId('mock-bar'));
    expect(onPointSelect).toHaveBeenCalledWith({ t: 1, v: 7 });
    expect(screen.getByTestId('chart-selected-pill')).toHaveTextContent('Selected: 7');
  });

  it('formats epoch ms timestamps as HH:MM:SS and step indices / compact numbers cleanly', () => {
    const epoch = Date.UTC(2026, 10, 14, 15, 4, 9);
    expect(formatChartXTick(epoch)).toMatch(/^\d{2}:\d{2}:\d{2}$/);
    expect(formatChartXTick(4)).toBe('4');
    expect(formatCompactMetricValue(1500)).toBe('1.5k');
    expect(formatCompactMetricValue(12.4, '$')).toBe('$12.4');
    expect(formatCompactMetricValue(85, '%')).toBe('85%');
  });
});

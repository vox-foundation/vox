import React from 'react';
import { ResponsiveContainer, Tooltip as RechartsTooltip } from 'recharts';
import { cn } from '../../lib/cn';

export type ChartConfig = Record<
  string,
  {
    label?: React.ReactNode;
    color?: string;
  }
>;

const ChartContext = React.createContext<{ config: ChartConfig }>({ config: {} });

export interface ChartContainerProps extends React.HTMLAttributes<HTMLDivElement> {
  config: ChartConfig;
  children: React.ComponentProps<typeof ResponsiveContainer>['children'];
}

const ChartContainer = React.forwardRef<HTMLDivElement, ChartContainerProps>(
  ({ config, className, style, children, ...props }, ref) => {
    const colorVars = React.useMemo(() => {
      const vars: Record<string, string> = {};
      for (const [key, item] of Object.entries(config)) {
        if (item.color) {
          vars[`--color-${key}`] = item.color;
        }
      }
      return vars;
    }, [config]);

    return (
      <ChartContext.Provider value={{ config }}>
        <div
          ref={ref}
          data-slot="chart"
          style={{ ...colorVars, ...style } as React.CSSProperties}
          className={cn('flex h-40 w-full min-h-[120px] min-w-0 justify-center text-xs', className)}
          {...props}
        >
          <ResponsiveContainer width="100%" height="100%">
            {children}
          </ResponsiveContainer>
        </div>
      </ChartContext.Provider>
    );
  }
);
ChartContainer.displayName = 'ChartContainer';

const ChartTooltip = RechartsTooltip;

export interface ChartTooltipPayloadEntry {
  name?: string;
  dataKey?: string | number;
  value?: number | string;
  color?: string;
}

export interface ChartTooltipContentProps {
  active?: boolean;
  label?: React.ReactNode;
  payload?: ChartTooltipPayloadEntry[];
  className?: string;
}

const ChartTooltipContent = ({ active, label, payload, className }: ChartTooltipContentProps) => {
  const { config } = React.useContext(ChartContext);
  if (!active || !payload || payload.length === 0) return null;

  return (
    <div
      data-slot="chart-tooltip"
      className={cn(
        'grid min-w-[8rem] gap-1.5 rounded-lg border border-border-strong bg-popover px-2.5 py-1.5 text-xs text-popover-foreground shadow-md',
        className
      )}
    >
      {label !== undefined && (
        <div className="font-display text-[11px] font-semibold text-text-primary">{label}</div>
      )}
      {payload.map((entry, idx) => {
        const key = String(entry.dataKey ?? entry.name ?? idx);
        const itemCfg = config[key];
        const indicatorColor = itemCfg?.color ?? entry.color ?? 'var(--color-chart-1)';
        return (
          <div key={key} className="flex items-center justify-between gap-3">
            <span className="inline-flex items-center gap-1.5 text-text-secondary">
              <span
                className="size-2 shrink-0 rounded-xs"
                style={{ backgroundColor: indicatorColor }}
                aria-hidden="true"
              />
              <span>{itemCfg?.label ?? entry.name ?? key}</span>
            </span>
            <span className="font-mono font-medium tabular-nums text-text-primary">
              {entry.value}
            </span>
          </div>
        );
      })}
    </div>
  );
};

export { ChartContainer, ChartTooltip, ChartTooltipContent };

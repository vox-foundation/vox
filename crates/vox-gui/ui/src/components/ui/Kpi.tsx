import React from 'react';
import { Glass } from './Glass';
import { Icon } from './Icons';
import { cn } from '../../lib/cn';

const ACCENT_COLORS = {
  cyan: 'text-[#8C6825]',
  amber: 'text-[#9A5812]',
  emerald: 'text-[#4D5D1E]',
  violet: 'text-[#7A5418]',
  brass: 'text-[#1C140C]',
  zinc: 'text-[#3B2E1E]',
  sky: 'text-[#8C6825]',
};

export interface KpiProps extends React.HTMLAttributes<HTMLDivElement> {
  label: string;
  value: string | number;
  unit?: string;
  delta?: number;
  trend?: 'up' | 'down' | 'flat';
  accent?: keyof typeof ACCENT_COLORS;
  sparkData?: number[];
  icon?: React.ReactNode;
  onClick?: () => void;
  children?: React.ReactNode;
  as?: React.ElementType;
}

export function Kpi({
  label,
  value,
  unit = '',
  delta,
  trend = 'flat',
  accent = 'brass',
  sparkData,
  icon,
  onClick,
  className,
  children,
  as,
  ...props
}: KpiProps) {
  const isClickable = !!onClick;
  const Comp = as || (isClickable ? 'button' : 'div');
  const gradId = React.useId();
  const strokeHex =
    accent === 'emerald'
      ? '#3F5E1D'
      : accent === 'amber'
        ? '#9A5812'
        : accent === 'violet'
          ? '#7A5418'
          : '#9A711C';
  const chartGeom = React.useMemo(() => {
    if (!sparkData || sparkData.length < 2) return null;
    const min = Math.min(...sparkData);
    const max = Math.max(...sparkData);
    const span = max - min || 1;
    const W = 200;
    const H = 26;
    const pts = sparkData.map((v, i) => ({
      x: (i / (sparkData.length - 1)) * (W - 2) + 1,
      y: H - 3 - ((v - min) / span) * (H - 7),
    }));
    const line = pts
      .map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x.toFixed(1)} ${p.y.toFixed(1)}`)
      .join(' ');
    const area = `${line} L ${(W - 1).toFixed(1)} ${H} L 1 ${H} Z`;
    return { line, area, W, H, last: pts[pts.length - 1]! };
  }, [sparkData]);
  
  return (
    <Glass
      as={Comp}
      size="sm"
      interactive={isClickable}
      onClick={onClick}
      style={{
        backgroundImage:
          "linear-gradient(180deg, rgba(248, 243, 232, 0.96) 0%, rgba(235, 226, 208, 0.94) 100%), url('/assets/romanesque/travertine-parchment-tablet.jpg')",
        backgroundSize: 'cover',
        backgroundPosition: 'center',
      }}
      className={cn(
        'vox-marble-tablet relative flex flex-col select-none overflow-hidden border-2 border-[#C59B27]/85 shadow-[0_8px_22px_rgba(0,0,0,0.48),inset_0_1px_0_rgba(255,255,255,0.92)]',
        className,
      )}
      {...props}
    >
      <div className="pointer-events-none absolute inset-x-0 top-0 h-[2px] bg-linear-to-r from-transparent via-[#C59B27] to-transparent" aria-hidden="true" />
      <div className="flex items-center justify-between gap-2">
        <span className="text-[10px] uppercase font-display tracking-[0.14em] text-[#5C441E] font-bold truncate">
          {label}
        </span>
        {icon && <span className="text-[#785312] flex shrink-0">{icon}</span>}
      </div>

      <div className="flex items-baseline gap-1 mt-1">
        <span className={cn("font-mono font-bold tracking-tight text-[18px] tabular-nums", ACCENT_COLORS[accent])}>
          {value}
        </span>
        {unit && <span className="text-xs text-[#5C441E] font-semibold">{unit}</span>}
        
        {delta !== undefined && (
          <span
            data-trend={trend === 'flat' ? 'neutral' : trend}
            aria-label={`Delta ${trend === 'up' ? 'up' : trend === 'down' ? 'down' : 'neutral'} ${Math.abs(delta)}`}
            className={cn(
              "ml-auto font-mono text-[10px] font-semibold inline-flex items-center gap-0.5 tabular-nums",
              trend === 'up' ? 'text-[#4D5D1E]' : trend === 'down' ? 'text-[#881B1B]' : 'text-[#5C441E]'
            )}
          >
            {trend === 'up' ? (
              <Icon.trendUp data-icon="trend-up" className="size-3 inline-block shrink-0" aria-hidden="true" />
            ) : trend === 'down' ? (
              <Icon.trendDown data-icon="trend-down" className="size-3 inline-block shrink-0" aria-hidden="true" />
            ) : (
              <Icon.minus data-icon="trend-flat" className="size-3 inline-block shrink-0" aria-hidden="true" />
            )}
            <span>{Math.abs(delta)}</span>
          </span>
        )}
      </div>
      <span className={cn('vox-metric-rule', ACCENT_COLORS[accent])} aria-hidden="true" />
      {chartGeom && (
        <div className="mt-1.5 w-full">
          <svg
            data-cv-sparkline="true"
            viewBox={`0 0 ${chartGeom.W} ${chartGeom.H}`}
            preserveAspectRatio="none"
            className="h-6 w-full block overflow-visible"
            aria-hidden="true"
          >
            <defs>
              <linearGradient id={`kpi-area-${gradId}`} x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={strokeHex} stopOpacity="0.34" />
                <stop offset="100%" stopColor={strokeHex} stopOpacity="0.03" />
              </linearGradient>
            </defs>
            <path d={chartGeom.area} fill={`url(#kpi-area-${gradId})`} />
            <path
              d={chartGeom.line}
              fill="none"
              stroke={strokeHex}
              strokeWidth="1.75"
              vectorEffect="non-scaling-stroke"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>
      )}

      {children}
    </Glass>
  );
}

Kpi.Sub = function KpiSub({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-[11px] text-[#4A371E] font-medium mt-1 leading-none select-text">
      {children}
    </div>
  );
};

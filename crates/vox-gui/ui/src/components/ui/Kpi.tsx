import React from 'react';
import { Glass } from './Glass';
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
          <span className={cn(
            "ml-auto font-mono text-[10px] font-semibold flex items-center tabular-nums",
            trend === 'up' ? 'text-[#4D5D1E]' : trend === 'down' ? 'text-[#881B1B]' : 'text-[#5C441E]'
          )}>
            {trend === 'up' ? '▲' : trend === 'down' ? '▼' : '■'}
            {Math.abs(delta)}
          </span>
        )}
        {sparkData && sparkData.length > 1 && (
          <svg
            viewBox="0 0 60 18"
            className={cn("ml-auto h-4 w-14 shrink-0 overflow-visible opacity-85", ACCENT_COLORS[accent])}
            aria-hidden="true"
          >
            <path
              d={(() => {
                const min = Math.min(...sparkData);
                const max = Math.max(...sparkData);
                const span = max - min || 1;
                return sparkData
                  .map((v, i) => {
                    const x = (i / (sparkData.length - 1)) * 56 + 2;
                    const y = 15 - ((v - min) / span) * 12;
                    return `${i === 0 ? 'M' : 'L'} ${x.toFixed(1)} ${y.toFixed(1)}`;
                  })
                  .join(' ');
              })()}
              fill="none"
              stroke="currentColor"
              strokeWidth="1.6"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        )}
      </div>
      <span className={cn('vox-metric-rule', ACCENT_COLORS[accent])} aria-hidden="true" />

      {children}
    </Glass>
  );
}

Kpi.Sub = function KpiSub({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-[11px] text-[#C8B99E] mt-1 leading-none select-text">
      {children}
    </div>
  );
};

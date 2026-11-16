import React, { useId, useRef, useState } from 'react';
import { cn } from '../../lib/cn';

export interface RomanRotaryKnobProps {
  ariaLabel: string;
  value?: number;
  defaultValue?: number;
  min?: number;
  max?: number;
  step?: number;
  detents?: number[];
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  label?: string;
  showReadout?: boolean;
  formatValue?: (value: number) => string;
  onValueChange?: (value: number) => void;
  onValueCommit?: (value: number) => void;
  className?: string;
}

const SIZE_PX: Record<NonNullable<RomanRotaryKnobProps['size']>, number> = {
  sm: 36,
  md: 48,
  lg: 60,
};

const SWEEP_PX = 140;
const START_DEG = -135;
const SWEEP_DEG = 270;

function polarToCartesian(cx: number, cy: number, r: number, angleDeg: number) {
  const rad = ((angleDeg - 90) * Math.PI) / 180;
  return { x: cx + r * Math.cos(rad), y: cy + r * Math.sin(rad) };
}

function describeArc(cx: number, cy: number, r: number, startAngle: number, endAngle: number): string {
  if (Math.abs(endAngle - startAngle) < 0.01) return '';
  const start = polarToCartesian(cx, cy, r, startAngle);
  const end = polarToCartesian(cx, cy, r, endAngle);
  const largeArc = endAngle - startAngle > 180 ? 1 : 0;
  return `M ${start.x.toFixed(2)} ${start.y.toFixed(2)} A ${r} ${r} 0 ${largeArc} 1 ${end.x.toFixed(2)} ${end.y.toFixed(2)}`;
}

function decimalPlaces(step: number): number {
  const s = step.toString().toLowerCase();
  if (s.includes('e-')) {
    const [coeff, expPart] = s.split('e-');
    const exp = Number.parseInt(expPart ?? '0', 10);
    const frac = (coeff?.split('.')[1] ?? '').length;
    return (Number.isFinite(exp) ? exp : 0) + frac;
  }
  return (s.split('.')[1] ?? '').length;
}

function roundToStep(raw: number, min: number, max: number, step: number): number {
  const clamped = Math.min(max, Math.max(min, raw));
  if (!Number.isFinite(step) || step <= 0) return clamped;
  const steps = Math.round((clamped - min) / step);
  const snapped = min + steps * step;
  const decimals = decimalPlaces(step);
  return Number(Math.min(max, Math.max(min, snapped)).toFixed(Math.min(12, decimals + 1)));
}

const RomanRotaryKnob = React.forwardRef<HTMLDivElement, RomanRotaryKnobProps>(
  (
    {
      ariaLabel,
      value,
      defaultValue,
      min = 0,
      max = 100,
      step = 1,
      detents,
      size = 'md',
      disabled = false,
      label,
      showReadout = true,
      formatValue,
      onValueChange,
      onValueCommit,
      className,
    },
    ref
  ) => {
    const safeMin = Number.isFinite(min) ? min : 0;
    const safeMax = Number.isFinite(max) && max > safeMin ? max : safeMin + 100;
    const safeStep = Number.isFinite(step) && step > 0 ? step : 1;
    const initialFallback = Math.min(safeMax, Math.max(safeMin, Number.isFinite(defaultValue) ? (defaultValue as number) : safeMin));

    const [uncontrolled, setUncontrolled] = useState<number>(initialFallback);
    const isControlled = value !== undefined && Number.isFinite(value);
    const currentValue = Math.min(safeMax, Math.max(safeMin, isControlled ? (value as number) : uncontrolled));

    const dragRef = useRef<{ active: boolean; lastY: number; rawValue: number; latest: number }>({
      active: false,
      lastY: 0,
      rawValue: currentValue,
      latest: currentValue,
    });
    const gradId = useId().replace(/:/g, '');

    const applyNext = (candidate: number, fineMode: boolean, commitNow: boolean) => {
      const quantStep = fineMode ? safeStep * 0.1 : safeStep;
      let next = roundToStep(candidate, safeMin, safeMax, quantStep);
      if (!fineMode && detents && detents.length > 0) {
        const snapTol = Math.max(safeStep * 1.25, (safeMax - safeMin) * 0.03);
        for (const d of detents) {
          if (Math.abs(next - d) <= snapTol) {
            next = Math.min(safeMax, Math.max(safeMin, d));
            break;
          }
        }
      }
      dragRef.current.latest = next;
      if (!isControlled) setUncontrolled(next);
      onValueChange?.(next);
      if (commitNow) onValueCommit?.(next);
    };

    const ratio = safeMax === safeMin ? 0 : (currentValue - safeMin) / (safeMax - safeMin);
    const angleDeg = START_DEG + ratio * SWEEP_DEG;
    const formatted = formatValue ? formatValue(currentValue) : String(currentValue);
    const px = SIZE_PX[size];
    const trackPath = describeArc(24, 24, 18.5, START_DEG, START_DEG + SWEEP_DEG);
    const activePath = describeArc(24, 24, 18.5, START_DEG, angleDeg);

    return (
      <div className={cn('inline-flex flex-col items-center gap-1 select-none', disabled && 'opacity-50', className)}>
        {label && (
          <span className="font-display text-[10px] tracking-wider text-text-muted uppercase">{label}</span>
        )}
        <div
          ref={ref}
          role="slider"
          tabIndex={disabled ? -1 : 0}
          data-slot="roman-rotary-knob"
          data-roman-gauge="true"
          aria-label={ariaLabel}
          aria-valuemin={safeMin}
          aria-valuemax={safeMax}
          aria-valuenow={currentValue}
          aria-valuetext={formatted}
          aria-disabled={disabled || undefined}
          style={{ width: px, height: px }}
          className="relative flex cursor-ns-resize items-center justify-center rounded-full touch-none outline-none focus-visible:outline-solid focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:cursor-not-allowed"
          onPointerDown={(e) => {
            if (disabled || e.button !== 0) return;
            e.preventDefault();
            e.currentTarget.focus();
            e.currentTarget.setPointerCapture?.(e.pointerId);
            dragRef.current = {
              active: true,
              lastY: e.clientY,
              rawValue: currentValue,
              latest: currentValue,
            };
          }}
          onPointerMove={(e) => {
            if (disabled || !dragRef.current.active) return;
            const dy = dragRef.current.lastY - e.clientY;
            dragRef.current.lastY = e.clientY;
            const multiplier = e.shiftKey ? 0.1 : 1;
            const nextRaw = Math.min(
              safeMax,
              Math.max(safeMin, dragRef.current.rawValue + (dy / SWEEP_PX) * (safeMax - safeMin) * multiplier),
            );
            dragRef.current.rawValue = nextRaw;
            applyNext(nextRaw, e.shiftKey, false);
          }}
          onPointerUp={(e) => {
            if (!dragRef.current.active) return;
            dragRef.current.active = false;
            try {
              e.currentTarget.releasePointerCapture?.(e.pointerId);
            } catch {
              // Pointer capture may already have been released
            }
            onValueCommit?.(dragRef.current.latest);
          }}
          onPointerCancel={(e) => {
            if (!dragRef.current.active) return;
            dragRef.current.active = false;
            try {
              e.currentTarget.releasePointerCapture?.(e.pointerId);
            } catch {
              // Pointer capture may already have been released
            }
          }}
          onLostPointerCapture={() => {
            if (!dragRef.current.active) return;
            dragRef.current.active = false;
            onValueCommit?.(dragRef.current.latest);
          }}
          onDoubleClick={() => {
            if (disabled) return;
            applyNext(initialFallback, false, true);
          }}
          onKeyDown={(e) => {
            if (disabled) return;
            const unit = e.shiftKey ? safeStep * 0.1 : safeStep;
            let next: number | null = null;
            if (e.key === 'ArrowUp' || e.key === 'ArrowRight') next = currentValue + unit;
            else if (e.key === 'ArrowDown' || e.key === 'ArrowLeft') next = currentValue - unit;
            else if (e.key === 'PageUp') next = currentValue + safeStep * 10;
            else if (e.key === 'PageDown') next = currentValue - safeStep * 10;
            else if (e.key === 'Home') next = safeMin;
            else if (e.key === 'End') next = safeMax;
            if (next !== null) {
              e.preventDefault();
              applyNext(next, e.shiftKey, true);
            }
          }}
        >
          <svg viewBox="0 0 48 48" className="size-full overflow-visible" aria-hidden="true">
            <defs>
              <radialGradient id={gradId} cx="50%" cy="35%" r="65%">
                <stop offset="0%" stopColor="var(--color-bg-surface)" />
                <stop offset="100%" stopColor="var(--color-bg-elevated)" />
              </radialGradient>
            </defs>
            {Array.from({ length: 11 }, (_, idx) => {
              const tickDeg = START_DEG + (idx / 10) * SWEEP_DEG;
              const p1 = polarToCartesian(24, 24, 21.2, tickDeg);
              const p2 = polarToCartesian(24, 24, 22.8, tickDeg);
              return (
                <line
                  key={idx}
                  x1={p1.x}
                  y1={p1.y}
                  x2={p2.x}
                  y2={p2.y}
                  stroke="var(--color-border-strong)"
                  strokeWidth={idx % 5 === 0 ? 1.4 : 0.9}
                  strokeLinecap="round"
                />
              );
            })}
            {detents?.map((d) => {
              const dRatio = safeMax === safeMin ? 0 : (d - safeMin) / (safeMax - safeMin);
              const dPoint = polarToCartesian(24, 24, 22, START_DEG + dRatio * SWEEP_DEG);
              return <circle key={d} cx={dPoint.x} cy={dPoint.y} r={1.3} fill="var(--color-primary)" />;
            })}
            <path
              d={trackPath}
              fill="none"
              stroke="var(--color-border-subtle)"
              strokeWidth={2.4}
              strokeLinecap="round"
            />
            {activePath && (
              <path
                d={activePath}
                fill="none"
                stroke="var(--color-primary)"
                strokeWidth={2.6}
                strokeLinecap="round"
              />
            )}
            <circle
              cx="24"
              cy="24"
              r="14.2"
              fill={`url(#${gradId})`}
              stroke="var(--color-border-strong)"
              strokeWidth={1.35}
            />
            <g transform={`rotate(${angleDeg.toFixed(2)} 24 24)`}>
              <line
                x1="24"
                y1="20"
                x2="24"
                y2="12"
                stroke="var(--color-primary)"
                strokeWidth={2}
                strokeLinecap="round"
              />
              <circle cx="24" cy="24" r="2.2" fill="var(--color-text-primary)" />
            </g>
          </svg>
        </div>
        {showReadout && (
          <span
            data-slot="roman-rotary-knob-readout"
            className="font-mono text-[11px] font-medium tabular-nums text-text-secondary"
          >
            {formatted}
          </span>
        )}
      </div>
    );
  }
);
RomanRotaryKnob.displayName = 'RomanRotaryKnob';

export { RomanRotaryKnob };

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
  md: 54,
  lg: 72,
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
      <div className={cn('inline-flex flex-col items-center gap-1.5 select-none', disabled && 'opacity-50', className)}>
        {label && (
          <div className="flex items-center gap-1.5">
            <span
              aria-hidden="true"
              className="size-2 rounded-full border border-[#785312]"
              style={{
                backgroundColor: ratio > 0 ? '#1B5E3A' : '#C59B27',
              }}
            />
            <span className="font-display text-[11px] font-bold tracking-[0.12em] text-text-primary uppercase">
              {label}
            </span>
          </div>
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
          className="relative flex cursor-ns-resize items-center justify-center rounded-full touch-none outline-none focus-visible:outline-solid focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:cursor-not-allowed drop-shadow-[0_3px_6px_rgba(120,83,18,0.28)]"
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
              <radialGradient id={gradId} cx="42%" cy="32%" r="68%">
                <stop offset="0%" stopColor="#FDF4CE" />
                <stop offset="48%" stopColor="#D9B45A" />
                <stop offset="85%" stopColor="#9A741C" />
                <stop offset="100%" stopColor="#6B4C12" />
              </radialGradient>
              <linearGradient id={`${gradId}-bezel`} x1="0" y1="0" x2="1" y2="1">
                <stop offset="0%" stopColor="#F2D67B" />
                <stop offset="50%" stopColor="#B88E28" />
                <stop offset="100%" stopColor="#6B4C12" />
              </linearGradient>
            </defs>
            {/* Precision outer graduation tick ring */}
            {Array.from({ length: 15 }, (_, idx) => {
              const tickDeg = START_DEG + (idx / 14) * SWEEP_DEG;
              const p1 = polarToCartesian(24, 24, 21.0, tickDeg);
              const p2 = polarToCartesian(24, 24, idx % 7 === 0 ? 23.8 : 22.8, tickDeg);
              return (
                <line
                  key={idx}
                  x1={p1.x}
                  y1={p1.y}
                  x2={p2.x}
                  y2={p2.y}
                  stroke={idx % 7 === 0 ? '#9A7014' : 'var(--color-border-strong)'}
                  strokeWidth={idx % 7 === 0 ? 1.5 : 0.9}
                  strokeLinecap="round"
                />
              );
            })}
            {detents?.map((d) => {
              const dRatio = safeMax === safeMin ? 0 : (d - safeMin) / (safeMax - safeMin);
              const dPoint = polarToCartesian(24, 24, 22, START_DEG + dRatio * SWEEP_DEG);
              return <circle key={d} cx={dPoint.x} cy={dPoint.y} r={1.4} fill="#C59B27" />;
            })}
            {/* Recessed dial arc track */}
            <path
              d={trackPath}
              fill="none"
              stroke="var(--color-border-subtle)"
              strokeWidth={2.6}
              strokeLinecap="round"
            />
            {activePath && (
              <path
                d={activePath}
                fill="none"
                stroke="#C59B27"
                strokeWidth={2.8}
                strokeLinecap="round"
              />
            )}
            {/* Outer knurled cast-brass collar */}
            <circle
              cx="24"
              cy="24"
              r="15.8"
              fill="none"
              stroke={`url(#${gradId}-bezel)`}
              strokeWidth={2.4}
              strokeDasharray="2.2 1.4"
            />
            {/* Brushed brass physical rotary dial cap */}
            <circle
              cx="24"
              cy="24"
              r="13.8"
              fill={`url(#${gradId})`}
              stroke="#785312"
              strokeWidth={1.3}
            />
            <circle
              cx="24"
              cy="24"
              r="11.2"
              fill="none"
              stroke="#FAF3DC"
              strokeOpacity="0.45"
              strokeWidth={0.8}
            />
            {/* Engraved pointer groove & central rivet */}
            <g transform={`rotate(${angleDeg.toFixed(2)} 24 24)`}>
              <line
                x1="24"
                y1="21"
                x2="24"
                y2="11.2"
                stroke="#4A330D"
                strokeWidth={2.5}
                strokeLinecap="round"
              />
              <line
                x1="24"
                y1="20.5"
                x2="24"
                y2="11.6"
                stroke="#FFF7D6"
                strokeOpacity="0.65"
                strokeWidth={0.9}
                strokeLinecap="round"
              />
              <circle cx="24" cy="24" r="2.4" fill="#4A330D" stroke="#F3E5AB" strokeWidth={0.7} />
            </g>
          </svg>
        </div>
        {showReadout && (
          <span
            data-slot="roman-rotary-knob-readout"
            className="rounded border border-border-strong bg-bg-elevated px-2 py-0.5 font-display text-[11px] font-bold tabular-nums text-text-primary shadow-xs"
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

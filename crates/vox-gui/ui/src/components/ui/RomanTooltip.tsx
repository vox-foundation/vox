import React, { useState, useRef, useEffect, useCallback } from 'react';
import { createPortal } from 'react-dom';

export interface RomanTooltipMetric {
  label: string;
  value: string;
  tone?: 'gold' | 'emerald' | 'crimson';
}

export interface RomanTooltipProps {
  /** Plain-English title of the feature, button, or surface */
  title: string;
  /** Plain-English explanation of what this feature is for */
  purpose: string;
  /** Plain-English description of what clicking/interacting does */
  actionHint?: string;
  /** Optional keyboard shortcut or badge */
  shortcut?: string;
  /** Optional live status pill text */
  statusBadge?: string;
  /** Optional bespoke Roman architectural building plate shown in the tooltip header */
  previewImageSrc?: string;
  /** Optional Roman tier / crate inscription tag shown over the architectural plate */
  crateBadge?: string;
  /** Optional structured live telemetry badges for interactive buildings */
  metrics?: readonly RomanTooltipMetric[];
  /** Optional external hover state (e.g. when hovering a building footprint on canvas) */
  forceOpen?: boolean;
  /** Preferred placement */
  placement?: 'top' | 'bottom' | 'right' | 'left';
  children: React.ReactElement;
  className?: string;
  disabled?: boolean;
}

/**
 * Plate 2 Romanesque Progressive-Disclosure & Architectural Monument Tooltip.
 * Renders a carved Carrara marble & burnished brass tablet in a portal explaining
 * what an interactive control or monument is for, its live activity status, and player guidance.
 */
export function RomanTooltip({
  title,
  purpose,
  actionHint,
  shortcut,
  statusBadge,
  previewImageSrc,
  crateBadge,
  metrics,
  forceOpen = false,
  placement = 'top',
  children,
  className = 'inline-flex',
  disabled = false,
}: RomanTooltipProps) {
  const [open, setOpen] = useState(false);
  const isVisible = (open || forceOpen) && !disabled;
  const [coords, setCoords] = useState<{ top: number; left: number }>({ top: 0, left: 0 });
  const triggerRef = useRef<HTMLSpanElement | null>(null);
  const tooltipRef = useRef<HTMLDivElement | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const tooltipWidth = previewImageSrc ? 304 : 276;

  const computePosition = useCallback(() => {
    const el = triggerRef.current;
    if (!el || typeof window === 'undefined') return;
    const rect = el.getBoundingClientRect();
    const measuredH = tooltipRef.current?.offsetHeight ?? 0;
    const estimatedH =
      (previewImageSrc ? 104 : 0) +
      (metrics && metrics.length > 0 ? 34 : 0) +
      (actionHint ? 124 : 88);
    const height = measuredH > 24 ? measuredH : estimatedH;
    const width = tooltipWidth;
    const pad = 12;

    let top = rect.top - height - 10;
    let left = rect.left + rect.width / 2 - width / 2;

    if (placement === 'bottom') {
      top = rect.bottom + 10;
    } else if (placement === 'right') {
      top = rect.top + rect.height / 2 - height / 2;
      left = rect.right + 12;
    } else if (placement === 'left') {
      top = rect.top + rect.height / 2 - height / 2;
      left = rect.left - width - 12;
    }

    if (top < pad) {
      top = Math.min(window.innerHeight - height - pad, rect.bottom + 10);
    }
    if (top + height > window.innerHeight - pad) {
      top = Math.max(pad, rect.top - height - 10);
    }
    if (top < pad) top = pad;
    if (left < pad) left = pad;
    if (left + width > window.innerWidth - pad) {
      left = Math.max(pad, window.innerWidth - width - pad);
    }

    setCoords({ top, left });
  }, [actionHint, metrics, placement, previewImageSrc, tooltipWidth]);

  const handleEnter = () => {
    if (disabled) return;
    if (timerRef.current) clearTimeout(timerRef.current);
    computePosition();
    timerRef.current = setTimeout(() => {
      computePosition();
      setOpen(true);
    }, 95);
  };

  const handleLeave = () => {
    if (timerRef.current) clearTimeout(timerRef.current);
    setOpen(false);
  };

  useEffect(() => {
    if (isVisible) {
      computePosition();
    }
  }, [isVisible, computePosition]);

  useEffect(() => {
    if (disabled && open) {
      if (timerRef.current) clearTimeout(timerRef.current);
      setOpen(false);
    }
  }, [disabled, open]);

  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  return (
    <span
      ref={triggerRef}
      onMouseEnter={handleEnter}
      onMouseLeave={handleLeave}
      onPointerDown={handleLeave}
      onClick={handleLeave}
      onFocus={handleEnter}
      onBlur={handleLeave}
      className={className}
    >
      {children}
      {isVisible &&
        typeof document !== 'undefined' &&
        createPortal(
          <div
            ref={tooltipRef}
            role="tooltip"
            data-testid="roman-progressive-tooltip"
            style={{ top: `${coords.top}px`, left: `${coords.left}px`, width: `${tooltipWidth}px` }}
            className="vox-marble-tablet fixed z-(--z-tooltip) pointer-events-none overflow-hidden rounded-xl border-2 border-[#C59B27] bg-bg-surface text-left text-text-primary shadow-[0_16px_36px_rgba(28,20,12,0.42),0_0_22px_rgba(212,175,55,0.28),inset_0_1px_0_rgba(255,255,255,0.95)] animate-in fade-in-0 zoom-in-95 duration-150"
          >
            {previewImageSrc && (
              <div className="relative h-28 w-full overflow-hidden border-b border-[#C59B27]/75">
                <img
                  src={previewImageSrc}
                  alt=""
                  aria-hidden="true"
                  data-testid="roman-tooltip-monument-art"
                  className="h-full w-full object-cover object-center"
                />
                <div
                  aria-hidden="true"
                  style={{
                    background:
                      'linear-gradient(to top, rgba(28,20,12,0.52) 0%, rgba(28,20,12,0.08) 46%, rgba(28,20,12,0) 100%)',
                  }}
                  className="pointer-events-none absolute inset-0"
                />
                <div className="absolute inset-x-2 bottom-1.5 flex items-center justify-between gap-1.5">
                  {crateBadge && (
                    <span className="rounded border border-border-strong bg-bg-surface px-1.5 py-0.5 font-mono text-[9px] font-bold tracking-wide text-text-primary shadow-sm">
                      {crateBadge}
                    </span>
                  )}
                  {statusBadge && (
                    <span className="inline-flex items-center gap-1 rounded border border-border-strong bg-bg-surface px-1.5 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider text-text-primary shadow-sm">
                      <span className="size-1.5 rounded-full bg-(--color-status-pass) animate-pulse" />
                      <span>{statusBadge}</span>
                    </span>
                  )}
                </div>
              </div>
            )}
            <div className="p-2.5">
              <div className="flex items-center justify-between gap-2 border-b border-border-subtle pb-1.5 mb-1.5">
                <span className="font-display text-[11px] font-bold uppercase tracking-[0.11em] text-text-primary leading-tight">
                  {title}
                </span>
                <div className="flex items-center gap-1 shrink-0">
                  {!previewImageSrc && statusBadge && (
                    <span className="rounded border border-(--color-status-pass) bg-(--color-status-pass)/15 px-1.5 py-0.5 font-display text-[9px] font-bold text-(--color-status-pass)">
                      {statusBadge}
                    </span>
                  )}
                  {shortcut && (
                    <kbd className="rounded border border-border-strong bg-bg-elevated px-1.5 py-0.5 font-mono text-[9px] font-bold text-text-primary">
                      {shortcut}
                    </kbd>
                  )}
                </div>
              </div>
              <p className="font-serif text-[11px] leading-snug text-text-secondary">{purpose}</p>
              {metrics && metrics.length > 0 && (
                <div className="mt-2 grid grid-cols-3 gap-1">
                  {metrics.map((m) => (
                    <div
                      key={m.label}
                      className="rounded border border-[#C59B27]/45 bg-bg-elevated px-1.5 py-1 text-center"
                    >
                      <div className="font-display text-[7.5px] font-bold uppercase tracking-wider text-text-muted">
                        {m.label}
                      </div>
                      <div
                        className={`font-mono text-[9px] font-bold leading-tight break-words ${
                          m.tone === 'emerald'
                            ? 'text-(--color-status-pass)'
                            : m.tone === 'crimson'
                              ? 'text-(--color-status-fail)'
                              : 'text-text-primary'
                        }`}
                      >
                        {m.value}
                      </div>
                    </div>
                  ))}
                </div>
              )}
              {actionHint && (
                <div className="mt-2 flex items-start gap-1.5 rounded-md border border-[#C59B27]/55 bg-bg-elevated px-2 py-1.5 text-[10px] text-text-primary">
                  <span className="mt-1 inline-block size-1.5 rounded-full bg-(--color-status-pass) animate-pulse shrink-0" />
                  <span className="font-semibold leading-snug">{actionHint}</span>
                </div>
              )}
            </div>
          </div>,
          document.body,
        )}
    </span>
  );
}

/**
 * Sunlit Gold & Cream Carrara / Cast-Bronze Medallion Badge (`<RomanMedallion>`).
 */
export function RomanMedallion({
  active = false,
  size = 'md',
  children,
  className = '',
}: {
  active?: boolean;
  size?: 'sm' | 'md' | 'lg';
  children: React.ReactNode;
  className?: string;
}) {
  const dims =
    size === 'sm' ? 'size-7' : size === 'lg' ? 'size-12' : 'size-8';
  return (
    <span
      data-roman-medallion="true"
      style={{
        backgroundImage: active
          ? 'radial-gradient(circle at 35% 30%, rgba(255,250,230,0.94) 0%, rgba(235,210,135,0.9) 62%, rgba(184,142,40,0.95) 100%)'
          : 'radial-gradient(circle at 35% 30%, rgba(250,246,238,0.95) 0%, rgba(236,226,207,0.92) 70%, rgba(212,188,142,0.95) 100%)',
      }}
      className={`relative inline-flex ${dims} shrink-0 items-center justify-center rounded-full border-2 transition-all duration-300 ${
        active
          ? 'vox-animate-brass-glint border-[#9A7014] text-text-primary shadow-[0_3px_10px_rgba(120,83,18,0.28),0_0_12px_rgba(212,175,55,0.45),inset_0_1px_2px_rgba(255,255,255,0.9)]'
          : 'border-[#B88E28] text-text-primary shadow-[0_2px_5px_rgba(120,83,18,0.18),inset_0_1px_2px_rgba(255,255,255,0.85)] group-hover:border-[#8C6825] group-hover:scale-105'
      } ${className}`}
    >
      {children}
    </span>
  );
}

/**
 * 3D Cast-Bronze / Obsidian Imperium Signet Coin (`<RomanCoinSignet>`)
 * positioned between semicircular brass chronometer dials in the architectural frieze header.
 */
export function RomanCoinSignet({
  className = '',
}: {
  label?: string;
  className?: string;
}) {
  return (
    <span
      data-roman-medallion="true"
      aria-hidden="true"
      className={`relative inline-flex size-12 sm:size-14 shrink-0 items-center justify-center overflow-hidden rounded-full border-2 border-[#B88E28] bg-bg-elevated p-0.5 shadow-[0_4px_12px_rgba(120,83,18,0.34),inset_0_1px_2px_rgba(255,250,225,0.75)] ${className}`}
    >
      <img
        src="/assets/romanesque/bronze-medallion-emblem.jpg"
        alt=""
        aria-hidden="true"
        className="vox-coin-portrait-carrara size-full rounded-full object-cover"
      />
      <img
        src="/assets/romanesque/v74-medallion-imperium-slate.jpg"
        alt=""
        aria-hidden="true"
        className="vox-coin-portrait-slate size-full rounded-full object-cover"
      />
      <span
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 rounded-full ring-1 ring-inset ring-[#F3E5AB]/55"
      />
    </span>
  );
}

/**
 * Plate 2 Physical Analog Semicircular Brass-Bezel Chronometer Dial (`<RomanBrassGauge>`)
 * with Roman-numeral tick scale, metallic brass progress arc, and pivoted needle.
 */
export function RomanBrassGauge({
  label,
  valueText,
  ratio,
  tone = 'gold',
  purpose,
  actionHint,
  active,
  onClick,
}: {
  label: string;
  valueText: string;
  ratio: number;
  tone?: 'gold' | 'emerald' | 'crimson';
  purpose?: string;
  actionHint?: string;
  active?: boolean;
  onClick?: () => void;
}) {
  const clamped = Math.max(0, Math.min(1, Number.isFinite(ratio) ? ratio : 0));
  const angleDeg = -84 + clamped * 168;
  const arcColor =
    tone === 'emerald'
      ? 'var(--color-status-pass)'
      : tone === 'crimson'
        ? 'var(--color-status-fail)'
        : 'var(--color-accent-default)';
  const needleColor =
    tone === 'emerald'
      ? 'var(--color-status-pass)'
      : tone === 'crimson'
        ? 'var(--color-status-fail)'
        : 'var(--color-text-primary)';

  const body = (
    <button
      type="button"
      data-roman-gauge="true"
      aria-pressed={onClick && active !== undefined ? active : undefined}
      onClick={onClick}
      className={`vox-chronometer-gauge vox-animate-dial-breathe group relative flex items-center gap-2 rounded-lg border-2 px-2.5 py-1.5 text-text-primary transition-[border-color,transform] duration-200 ${
        active
          ? 'border-(--color-accent-default) ring-1 ring-(--color-accent-default)/55'
          : 'border-border-strong'
      } ${
        onClick ? 'cursor-pointer hover:border-(--color-accent-default) hover:-translate-y-0.5' : 'cursor-default'
      }`}
    >
      <svg viewBox="0 0 76 42" className="w-16 h-9 shrink-0 overflow-visible" aria-hidden="true">
        {/* Outer cast-brass bevel arch & recessed chronometer dial face */}
        <path
          d="M 5 37 A 33 33 0 0 1 71 37 Z"
          fill="var(--color-bg-elevated)"
          stroke="var(--color-border-strong)"
          strokeWidth="2.4"
        />
        <path
          d="M 9 36 A 29 29 0 0 1 67 36"
          fill="none"
          stroke="#C59B27"
          strokeOpacity="0.45"
          strokeWidth="1"
        />
        {/* Recessed track */}
        <path
          d="M 12 36 A 26 26 0 0 1 64 36"
          fill="none"
          stroke="var(--color-border-subtle)"
          strokeWidth="4.6"
          strokeLinecap="round"
        />
        {/* Active telemetry arc */}
        <path
          d="M 12 36 A 26 26 0 0 1 64 36"
          fill="none"
          stroke={arcColor}
          strokeWidth="4.6"
          strokeLinecap="round"
          strokeDasharray="81.7"
          strokeDashoffset={81.7 * (1 - clamped)}
          className="transition-[stroke-dashoffset] duration-700 ease-out"
        />
        {/* Engraved Roman chronometer tick marks */}
        {[-78, -60, -42, -24, -6, 12, 30, 48, 66, 78].map((deg, idx) => (
          <g key={deg} transform={`translate(38, 36) rotate(${deg})`}>
            <line
              x1="0"
              y1={idx % 3 === 0 ? -24 : -26}
              x2="0"
              y2="-30"
              stroke="var(--color-text-muted)"
              strokeWidth={idx % 3 === 0 ? 1.35 : 0.85}
            />
          </g>
        ))}
        {/* Tapered sweeping brass needle */}
        <g
          transform={`translate(38, 36) rotate(${angleDeg})`}
          className="transition-transform duration-700 ease-out"
        >
          <polygon points="-1.6,0 0,-25 1.6,0" fill={needleColor} />
          <circle cx="0" cy="0" r="3.8" fill="#C59B27" stroke="var(--color-text-primary)" strokeWidth="1.1" />
        </g>
      </svg>
      <div className="flex flex-col text-left leading-tight">
        <span className="font-display text-[11px] font-bold uppercase tracking-[0.12em] text-text-muted whitespace-nowrap">
          {label}
        </span>
        <span className="font-display text-xs font-bold text-text-primary whitespace-nowrap">
          {valueText}
        </span>
      </div>
    </button>
  );

  if (!purpose && !actionHint) return body;
  return (
    <RomanTooltip
      title={`${label}: ${valueText}`}
      purpose={purpose ?? `${label} telemetry`}
      actionHint={actionHint}
    >
      {body}
    </RomanTooltip>
  );
}

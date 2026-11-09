import React, { useState, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';

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
  /** Preferred placement */
  placement?: 'top' | 'bottom' | 'right' | 'left';
  children: React.ReactElement;
  className?: string;
  disabled?: boolean;
}

/**
 * Plate 2 Romanesque Progressive-Disclosure Tooltip.
 * Renders a carved Carrara marble & burnished brass tablet in a portal explaining
 * what an interactive control is for and what clicking it does in plain English.
 */
export function RomanTooltip({
  title,
  purpose,
  actionHint,
  shortcut,
  statusBadge,
  placement = 'top',
  children,
  className = 'inline-flex',
  disabled = false,
}: RomanTooltipProps) {
  const [open, setOpen] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number }>({ top: 0, left: 0 });
  const triggerRef = useRef<HTMLSpanElement | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const computePosition = () => {
    const el = triggerRef.current;
    if (!el || typeof window === 'undefined') return;
    const rect = el.getBoundingClientRect();
    const width = 268;
    const height = actionHint ? 112 : 84;
    const pad = 10;

    let top = rect.top - height - 8;
    let left = rect.left + rect.width / 2 - width / 2;

    if (placement === 'bottom') {
      top = rect.bottom + 8;
    } else if (placement === 'right') {
      top = rect.top + rect.height / 2 - height / 2;
      left = rect.right + 10;
    } else if (placement === 'left') {
      top = rect.top + rect.height / 2 - height / 2;
      left = rect.left - width - 10;
    }

    if (top < pad) top = Math.min(window.innerHeight - height - pad, rect.bottom + 8);
    if (top + height > window.innerHeight - pad) top = Math.max(pad, rect.top - height - 8);
    if (left < pad) left = pad;
    if (left + width > window.innerWidth - pad) left = Math.max(pad, window.innerWidth - width - pad);

    setCoords({ top, left });
  };

  const handleEnter = () => {
    if (disabled) return;
    if (timerRef.current) clearTimeout(timerRef.current);
    computePosition();
    timerRef.current = setTimeout(() => {
      computePosition();
      setOpen(true);
    }, 140);
  };

  const handleLeave = () => {
    if (timerRef.current) clearTimeout(timerRef.current);
    setOpen(false);
  };

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
      {open &&
        !disabled &&
        typeof document !== 'undefined' &&
        createPortal(
          <div
            role="tooltip"
            data-testid="roman-progressive-tooltip"
            style={{ top: `${coords.top}px`, left: `${coords.left}px`, width: '268px' }}
            className="vox-marble-tablet fixed z-[99999] pointer-events-none rounded-lg border-2 border-border-strong bg-bg-surface p-2.5 text-left text-text-primary shadow-[0_12px_28px_rgba(28,20,12,0.35),inset_0_1px_0_rgba(255,255,255,0.95)] animate-in fade-in-0 zoom-in-95 duration-150"
          >
            <div className="flex items-center justify-between gap-2 border-b border-border-subtle pb-1.5 mb-1.5">
              <span className="font-display text-[11px] font-bold uppercase tracking-[0.12em] text-text-primary truncate">
                {title}
              </span>
              <div className="flex items-center gap-1 shrink-0">
                {statusBadge && (
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
            <p className="font-serif text-[11.5px] leading-snug text-text-secondary">{purpose}</p>
            {actionHint && (
              <div className="mt-1.5 flex items-center gap-1.5 rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[10px] text-text-primary">
                <span className="inline-block size-1.5 rounded-full bg-(--color-status-pass) animate-pulse shrink-0" />
                <span className="truncate font-semibold">{actionHint}</span>
              </div>
            )}
          </div>,
          document.body,
        )}
    </span>
  );
}

/**
 * Sunlit Gold & Cream Carrara Marble Medallion Badge (`<RomanMedallion>`).
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
    size === 'sm' ? 'size-7' : size === 'lg' ? 'size-11' : 'size-8';
  return (
    <span
      data-roman-medallion="true"
      style={{
        backgroundImage: active
          ? 'radial-gradient(circle, rgba(255,250,230,0.92) 0%, rgba(235,210,135,0.88) 68%, rgba(197,155,39,0.92) 100%), url(/assets/romanesque/carrara-seamless-stone.jpg)'
          : 'radial-gradient(circle, rgba(250,246,238,0.94) 0%, rgba(236,226,207,0.92) 72%, rgba(218,202,172,0.94) 100%), url(/assets/romanesque/carrara-seamless-stone.jpg)',
        backgroundSize: 'cover',
        backgroundPosition: 'center',
      }}
      className={`relative inline-flex ${dims} shrink-0 items-center justify-center rounded-full border-2 transition-all duration-300 ${
        active
          ? 'vox-animate-brass-glint border-[#9A7014] text-[#1C140C] shadow-[0_3px_10px_rgba(28,20,12,0.28),0_0_12px_rgba(212,175,55,0.45),inset_0_1px_2px_rgba(255,255,255,0.9)]'
          : 'border-[#B88E28] text-[#2B1E10] shadow-[0_2px_5px_rgba(28,20,12,0.18),inset_0_1px_2px_rgba(255,255,255,0.85)] group-hover:border-[#8C6825] group-hover:scale-105'
      } ${className}`}
    >
      {children}
    </span>
  );
}

/**
 * Plate 2 Physical Analog Semicircular Brass-Bezel Dial Gauge with Cream Carrara Marble Face,
 * Engraved Tick Marks, and Animated Sweeping Needle (matching `v61_concept_compute_and_market`).
 */
export function RomanBrassGauge({
  label,
  valueText,
  ratio,
  tone = 'gold',
  purpose,
  onClick,
}: {
  label: string;
  valueText: string;
  ratio: number;
  tone?: 'gold' | 'emerald' | 'crimson';
  purpose?: string;
  onClick?: () => void;
}) {
  const clamped = Math.max(0, Math.min(1, Number.isFinite(ratio) ? ratio : 0));
  const angleDeg = -86 + clamped * 172;
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
      onClick={onClick}
      className={`vox-animate-dial-breathe group relative flex items-center gap-2 rounded-lg border-2 border-border-strong bg-bg-surface px-2.5 py-1 text-text-primary shadow-[0_3px_10px_rgba(0,0,0,0.22),inset_0_1px_0_rgba(255,255,255,0.25)] transition-all duration-200 ${
        onClick ? 'cursor-pointer hover:border-(--color-accent-default) hover:-translate-y-0.5' : 'cursor-default'
      }`}
    >
      <svg viewBox="0 0 56 32" className="w-13 h-7.5 shrink-0 overflow-visible" aria-hidden="true">
        {/* Outer cast brass bezel & cream Carrara marble dial face */}
        <path
          d="M 4 28 A 24 24 0 0 1 52 28 Z"
          fill="var(--color-bg-elevated)"
          stroke="var(--color-border-strong)"
          strokeWidth="2.2"
        />
        {/* Recessed inner track */}
        <path
          d="M 8 27 A 20 20 0 0 1 48 27"
          fill="none"
          stroke="var(--color-border-subtle)"
          strokeWidth="4.2"
          strokeLinecap="round"
        />
        {/* Active progress arc */}
        <path
          d="M 8 27 A 20 20 0 0 1 48 27"
          fill="none"
          stroke={arcColor}
          strokeWidth="4.2"
          strokeLinecap="round"
          strokeDasharray="62.8"
          strokeDashoffset={62.8 * (1 - clamped)}
          className="transition-all duration-700 ease-out"
        />
        {/* Engraved Roman dial tick marks */}
        {[-75, -45, -15, 15, 45, 75].map((deg) => (
          <g key={deg} transform={`translate(28, 27) rotate(${deg})`}>
            <line x1="0" y1="-20" x2="0" y2="-23" stroke="var(--color-text-muted)" strokeWidth="1" />
          </g>
        ))}
        {/* Animated sweeping needle */}
        <g
          transform={`translate(28, 27) rotate(${angleDeg})`}
          className="transition-transform duration-700 ease-out"
        >
          <polygon points="-1.4,0 0,-19 1.4,0" fill={needleColor} />
          <circle cx="0" cy="0" r="3.2" fill="var(--color-accent-default)" stroke="var(--color-text-primary)" strokeWidth="1" />
        </g>
      </svg>
      <div className="flex flex-col text-left leading-tight">
        <span className="font-display text-[8px] font-bold uppercase tracking-[0.14em] text-text-muted">
          {label}
        </span>
        <span className="font-display text-[11px] font-bold text-text-primary whitespace-nowrap">
          {valueText}
        </span>
      </div>
    </button>
  );

  if (!purpose) return body;
  return (
    <RomanTooltip title={`${label}: ${valueText}`} purpose={purpose}>
      {body}
    </RomanTooltip>
  );
}

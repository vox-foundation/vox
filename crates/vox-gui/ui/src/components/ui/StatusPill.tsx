import React from 'react';
import { STATUS_TONE, StatusToneKind } from '../../styles/tokens';
import { Icon } from './Icons';
import { cn } from '../../lib/cn';

export interface StatusPillProps extends React.HTMLAttributes<HTMLSpanElement> {
  tone: StatusToneKind;
  label?: string;
  size?: 'xs' | 'sm';
  pulse?: boolean;
  icon?: React.ReactNode;
}

const DEFAULT_GLYPHS: Record<StatusToneKind, React.ReactNode> = {
  pass: <Icon.check className="size-3 shrink-0" aria-hidden="true" />,
  fail: '!',
  warn: '?',
  info: 'i',
  neutral: '·',
  accent: <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />,
  Executing: <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />,
  Verifying: <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />,
  Planning: <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />,
  Paused: '·',
  Validated: <Icon.check className="size-3 shrink-0" aria-hidden="true" />,
  Doubted: '?',
  Speculative: <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />,
  Active: <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />,
  Root: <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />,
};

const SIZE_CLASS = {
  xs: 'px-1.5 py-px text-[11px]',
  sm: 'px-2 py-0.5 text-[11px]',
};

export function StatusPill({ 
  tone, 
  label, 
  size = 'sm', 
  pulse = false, 
  icon,
  className,
  ...rest
}: StatusPillProps) {
  const toneStyle = STATUS_TONE[tone] || STATUS_TONE.neutral;
  const glyph = DEFAULT_GLYPHS[tone] || '·';
  
  // Auto-pulse Executing/Active/Verifying states
  const shouldPulse = pulse || tone === 'Executing' || tone === 'Active' || tone === 'Verifying';

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full font-medium tracking-[0.08em] uppercase ring-1 bg-overlay-subtle shrink-0 select-none",
        toneStyle.ring,
        toneStyle.text,
        SIZE_CLASS[size],
        className
      )}
      {...rest}
    >
      <span className={cn("relative inline-block size-1.5 rounded-full", toneStyle.dot)}>
        {shouldPulse && (
          <span className={cn("absolute inset-0 rounded-full animate-vox-ping opacity-60", toneStyle.dot)} />
        )}
      </span>
      {icon || (typeof glyph === 'string' ? <span className="font-mono" aria-hidden="true">{glyph}</span> : glyph)}
      <span>{label || tone}</span>
    </span>
  );
}

import React from 'react';
import { cn } from '../../../lib/cn';

export type CapabilityStatus = 'dedicated' | 'generic_form' | 'cli_only' | 'mcp_only';

export interface CapabilityBadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  status: CapabilityStatus;
}

interface BadgeConfig {
  label: string;
  glyph: string;
  className: string;
}

const BADGE_CONFIGS: Record<CapabilityStatus, BadgeConfig> = {
  dedicated: {
    label: 'Dedicated surface',
    glyph: '✓',
    className: 'text-(--color-status-pass) ring-(--color-status-pass)/30',
  },
  generic_form: {
    label: 'Generic form',
    glyph: '≡',
    className: 'text-(--color-status-info) ring-(--color-status-info)/30',
  },
  cli_only: {
    label: 'CLI-only',
    glyph: '>_',
    className: 'text-(--color-text-secondary) ring-(--color-text-secondary)/30',
  },
  mcp_only: {
    label: 'MCP-only / orphan',
    glyph: '◇',
    className: 'text-(--color-status-warn) ring-(--color-status-warn)/30',
  },
};

const BASE_RECIPE =
  'inline-flex items-center gap-1 rounded-full px-2 py-1 text-[11px] font-medium uppercase tracking-[0.08em] ring-1 bg-overlay-subtle shrink-0';

export function CapabilityBadge({ status, className, ...rest }: CapabilityBadgeProps) {
  const config = BADGE_CONFIGS[status];
  return (
    <span className={cn(BASE_RECIPE, config.className, className)} {...rest}>
      <span className="font-mono" aria-hidden="true">
        {config.glyph}
      </span>
      <span>{config.label}</span>
    </span>
  );
}

export type CapabilityTagKind = 'exempt' | 'overdue';

export interface CapabilityTagProps extends React.HTMLAttributes<HTMLSpanElement> {
  tag: CapabilityTagKind;
}

interface TagConfig {
  label: string;
  glyph?: string;
  className: string;
}

const TAG_CONFIGS: Record<CapabilityTagKind, TagConfig> = {
  exempt: {
    label: 'Exempt',
    className: 'text-(--color-text-secondary) ring-(--color-text-secondary)/30',
  },
  overdue: {
    label: 'Review overdue',
    glyph: '!',
    className: 'text-(--color-status-fail) ring-(--color-status-fail)/30',
  },
};

export function CapabilityTag({ tag, className, ...rest }: CapabilityTagProps) {
  const config = TAG_CONFIGS[tag];
  return (
    <span className={cn(BASE_RECIPE, config.className, className)} {...rest}>
      {config.glyph && (
        <span className="font-mono" aria-hidden="true">
          {config.glyph}
        </span>
      )}
      <span>{config.label}</span>
    </span>
  );
}

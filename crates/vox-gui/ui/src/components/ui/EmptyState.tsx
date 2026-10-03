import React from 'react';
import { Button } from './Button';
import { Icon } from './Icons';

export interface EmptyStateProps {
  variant?: 'no-data' | 'no-permission' | 'no-connection' | 'error' | 'welcome';
  icon?: React.ReactNode;
  title: string;
  description?: string;
  primaryAction?: { label: string; onClick: () => void };
  secondaryAction?: { label: string; onClick: () => void };
  action?: { label: string; onClick: () => void };
  children?: React.ReactNode;
  /** Heading level of the title: 2 when the surface's own h1 is the only heading above it (axe heading-order). */
  headingLevel?: 2 | 3;
}

const DEFAULT_ICONS = {
  'no-data': <Icon.alert className="size-8 text-text-muted" />,
  'no-permission': <Icon.x className="size-8 text-red-400" />,
  'no-connection': <Icon.bolt className="size-8 text-amber-400" />,
  'error': <Icon.alert className="size-8 text-red-500" />,
  'welcome': <Icon.check className="size-8 text-brass animate-pulse" />,
};

export function EmptyState({ 
  variant = 'no-data', 
  icon, 
  title, 
  description, 
  primaryAction, 
  secondaryAction,
  action,
  children,
  headingLevel = 3,
}: EmptyStateProps) {
  const actualPrimary = primaryAction || action;
  const Heading = headingLevel === 2 ? 'h2' : 'h3';

  return (
    <div
      className="flex flex-col items-center justify-center gap-3 py-16 px-4 text-center max-w-lg mx-auto"
      role="status"
      aria-live="polite"
    >
      <div className="flex justify-center mb-1">
        {icon || DEFAULT_ICONS[variant]}
      </div>
      <Heading className="font-display text-sm tracking-widest uppercase text-text-secondary">{title}</Heading>
      {description && <p className="text-xs text-text-muted leading-relaxed max-w-sm">{description}</p>}
      
      {children}

      {(actualPrimary || secondaryAction) && (
        <div className="flex items-center justify-center gap-3 mt-3">
          {secondaryAction && (
            <Button variant="ghost" size="sm" onClick={secondaryAction.onClick}>
              {secondaryAction.label}
            </Button>
          )}
          {actualPrimary && (
            <Button variant="primary" size="sm" onClick={actualPrimary.onClick}>
              {actualPrimary.label}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}


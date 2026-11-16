import React from 'react';
import type { LucideIcon, LucideProps } from 'lucide-react';
import { cn } from '../../lib/cn';

export interface VoxIconProps extends Omit<LucideProps, 'ref'> {
  icon: LucideIcon;
  name?: string;
}

/**
 * Stroke-normalized Lucide icon wrapper (`strokeWidth={1.5}`, `shrink-0`) matching
 * Vox Axis Romanesque `Icons.tsx` 24x24 architectural line weight.
 */
export function VoxIcon({
  icon: IconComponent,
  name,
  strokeWidth = 1.5,
  className,
  'aria-label': ariaLabel,
  ...props
}: VoxIconProps) {
  const isDecorative = !ariaLabel && props.role !== 'img';
  return (
    <IconComponent
      strokeWidth={strokeWidth}
      data-slot="vox-icon"
      data-icon={name ?? IconComponent.displayName ?? undefined}
      aria-hidden={isDecorative ? true : undefined}
      aria-label={ariaLabel}
      className={cn('shrink-0', className)}
      {...props}
    />
  );
}

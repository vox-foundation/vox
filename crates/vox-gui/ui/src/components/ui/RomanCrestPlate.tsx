import React, { useEffect, useState } from 'react';
import { cn } from '../../lib/cn';
import { isMinimalistDarkTheme } from '../../lib/theme';

export type RomanCrestVariant = 'empty' | 'imperium';

export interface RomanCrestPlateProps {
  variant?: RomanCrestVariant;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
  alt?: string;
}

const SIZE_CLASSES: Record<NonNullable<RomanCrestPlateProps['size']>, { box: string; px: number }> = {
  sm: { box: 'size-9', px: 36 },
  md: { box: 'size-14', px: 56 },
  lg: { box: 'size-20', px: 80 },
};

function readDarkSlateMode(): boolean {
  if (typeof document === 'undefined' || !document.documentElement) return false;
  return isMinimalistDarkTheme(document.documentElement.dataset.theme);
}

/**
 * Dual-theme sovereign relief medallion plate (`arcane`/`glacier` -> sunlit Carrara + brass;
 * `slate`/`void`/`high-contrast` -> precision obsidian basalt + anodized cyan).
 */
export function RomanCrestPlate({
  variant = 'empty',
  size = 'md',
  className,
  alt = '',
}: RomanCrestPlateProps) {
  const [isDarkSlate, setIsDarkSlate] = useState<boolean>(readDarkSlateMode);

  useEffect(() => {
    if (typeof document === 'undefined' || !document.documentElement || typeof MutationObserver === 'undefined') {
      return;
    }
    setIsDarkSlate(readDarkSlateMode());
    const observer = new MutationObserver(() => {
      setIsDarkSlate(readDarkSlateMode());
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    });
    return () => observer.disconnect();
  }, []);

  const src =
    variant === 'imperium'
      ? isDarkSlate
        ? '/assets/romanesque/v74-medallion-imperium-slate.jpg'
        : '/assets/romanesque/v74-medallion-imperium-carrara.jpg'
      : isDarkSlate
        ? '/assets/romanesque/v74-crest-empty-slate.jpg'
        : '/assets/romanesque/v74-crest-empty-carrara.jpg';

  const dim = SIZE_CLASSES[size];
  const isDecorative = alt.trim().length === 0;

  return (
    <span
      data-slot="roman-crest-plate"
      data-crest-variant={variant}
      aria-hidden={isDecorative ? true : undefined}
      className={cn(
        'inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full border border-border-subtle bg-bg-elevated shadow-sm',
        dim.box,
        className,
      )}
    >
      <img
        src={src}
        alt={alt}
        width={dim.px}
        height={dim.px}
        loading="lazy"
        decoding="async"
        className="h-full w-full object-cover select-none pointer-events-none"
      />
    </span>
  );
}

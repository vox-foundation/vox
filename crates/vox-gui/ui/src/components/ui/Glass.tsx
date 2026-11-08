import React from 'react';
import { cn } from '../../lib/cn';

const SIZE_PADDING = {
  sm: 'p-3 rounded-xl',
  md: 'p-5 rounded-2xl',
  lg: 'p-6 rounded-3xl',
};

export interface GlassProps extends React.HTMLAttributes<HTMLDivElement> {
  size?: keyof typeof SIZE_PADDING;
  inset?: boolean;
  interactive?: boolean;
  as?: React.ElementType;
  glow?: boolean;
  accent?: string;
  padding?: string;
  ariaLabel?: string;
}

export function Glass({ 
  className, 
  size = 'md',
  inset = true, 
  interactive = false,
  as: Comp = 'div',
  glow = false,
  accent,
  padding,
  ariaLabel,
  children, 
  ...rest 
}: GlassProps) {
  const isButton = Comp === 'button';
  const buttonProps = isButton ? { type: 'button' as const } : {};

  return (
    <Comp
      {...buttonProps}
      {...(ariaLabel ? { 'aria-label': ariaLabel } : {})}
      {...rest}
      className={cn(
        !isButton && "vox-marble-tablet",
        "relative border-2 border-[#C59B27]/80 bg-overlay-solid bg-linear-to-b from-[#FAF6EE]/96 via-[#F5EFE2]/96 to-[#EFE5D2]/96 text-[#1C140C] backdrop-blur-2xl shadow-[0_1px_0_rgba(255,255,255,0.92)_inset,0_10px_24px_-12px_rgba(28,20,12,0.28)]",
        isButton && "text-left bg-transparent border-0 p-0 outline-hidden w-full",
        padding ? padding : SIZE_PADDING[size],
        interactive && "hover:border-[#B8860B] hover:from-[#FFFDF9]/98 hover:to-[#F3EAD8]/98 hover:bg-bg-elevated cursor-pointer transition-[background-color,border-color,transform] duration-150 active:scale-[0.99] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-brass/40",
        glow && "shadow-[0_0_24px_rgba(197,155,39,0.28)]",
        className
      )}
    >
      {inset && (
        <div className="pointer-events-none absolute inset-0 rounded-[inherit] ring-1 ring-inset ring-[#D4AF37]/10" />
      )}
      {children}
    </Comp>
  );
}


import React from 'react';
import * as RadixTooltip from '@radix-ui/react-tooltip';

export interface TooltipProps {
  content: React.ReactNode;
  children: React.ReactNode;
  side?: 'top' | 'right' | 'bottom' | 'left';
  align?: 'start' | 'center' | 'end';
  sideOffset?: number;
  delayDuration?: number;
  className?: string;
}

export function Tooltip({
  content,
  children,
  side = 'top',
  align = 'center',
  sideOffset = 6,
  delayDuration = 150,
  className = '',
}: TooltipProps) {
  const [open, setOpen] = React.useState(false);
  if (!content) return <>{children}</>;

  return (
    <RadixTooltip.Provider delayDuration={delayDuration}>
      <RadixTooltip.Root open={open} onOpenChange={setOpen}>
        <RadixTooltip.Trigger
          asChild
          onPointerDown={() => setOpen(false)}
          onClick={() => setOpen(false)}
        >
          {children}
        </RadixTooltip.Trigger>
        <RadixTooltip.Portal>
          <RadixTooltip.Content
            side={side}
            align={align}
            sideOffset={sideOffset}
            onPointerDown={() => setOpen(false)}
            className={`z-[99999] max-w-xs rounded-md border border-[#C59B27]/55 bg-bg-elevated px-2.5 py-1 text-xs text-text-primary shadow-[0_10px_24px_rgba(62,45,24,0.16),0_0_0_1px_rgba(255,252,245,0.85)_inset] backdrop-blur-md animate-in fade-in-0 zoom-in-95 pointer-events-none select-none ${className}`}
          >
            {content}
            <RadixTooltip.Arrow className="fill-[var(--color-bg-elevated)]" />
          </RadixTooltip.Content>
        </RadixTooltip.Portal>
      </RadixTooltip.Root>
    </RadixTooltip.Provider>
  );
}

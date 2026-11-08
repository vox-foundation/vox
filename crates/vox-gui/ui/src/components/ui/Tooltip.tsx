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
            className={`z-[99999] max-w-xs rounded-md border border-[#D4AF37]/50 bg-linear-to-b from-[#1B1610]/98 via-[#14100C]/98 to-[#0E0B08]/98 px-2.5 py-1 text-xs text-[#F5F0E6] shadow-[0_14px_34px_rgba(0,0,0,0.88),0_0_0_1px_rgba(212,175,55,0.22)_inset] backdrop-blur-md animate-in fade-in-0 zoom-in-95 pointer-events-none select-none ${className}`}
          >
            {content}
            <RadixTooltip.Arrow className="fill-[#14100C]" />
          </RadixTooltip.Content>
        </RadixTooltip.Portal>
      </RadixTooltip.Root>
    </RadixTooltip.Provider>
  );
}

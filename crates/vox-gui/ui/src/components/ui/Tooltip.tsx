import React from 'react';
import { Tooltip as TooltipPrimitive } from 'radix-ui';
import { cn } from '../../lib/cn';

const TooltipProviderPresentContext = React.createContext(false);

export interface TooltipProviderProps {
  children: React.ReactNode;
  delayDuration?: number;
  skipDelayDuration?: number;
}

function TooltipProvider({
  children,
  delayDuration = 150,
  skipDelayDuration = 300,
}: TooltipProviderProps) {
  return (
    <TooltipProviderPresentContext.Provider value={true}>
      <TooltipPrimitive.Provider delayDuration={delayDuration} skipDelayDuration={skipDelayDuration}>
        {children}
      </TooltipPrimitive.Provider>
    </TooltipProviderPresentContext.Provider>
  );
}

export interface TooltipProps {
  content: React.ReactNode;
  children: React.ReactNode;
  side?: 'top' | 'right' | 'bottom' | 'left';
  align?: 'start' | 'center' | 'end';
  sideOffset?: number;
  delayDuration?: number;
  className?: string;
}

function Tooltip({
  content,
  children,
  side = 'top',
  align = 'center',
  sideOffset = 6,
  delayDuration = 150,
  className = '',
}: TooltipProps) {
  const hasRootProvider = React.useContext(TooltipProviderPresentContext);
  const [open, setOpen] = React.useState(false);

  if (!content) return <>{children}</>;

  const body = (
    <TooltipPrimitive.Root open={open} onOpenChange={setOpen} delayDuration={delayDuration}>
      <TooltipPrimitive.Trigger
        asChild
        data-slot="tooltip-trigger"
        onPointerDown={() => setOpen(false)}
        onClick={() => setOpen(false)}
      >
        {children}
      </TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content
          data-slot="tooltip-content"
          side={side}
          align={align}
          sideOffset={sideOffset}
          collisionPadding={8}
          onPointerDown={() => setOpen(false)}
          className={cn(
            'z-(--z-tooltip) max-w-xs rounded-md border border-border-strong bg-popover px-2.5 py-1 text-xs text-popover-foreground shadow-md backdrop-blur-md animate-in fade-in-0 zoom-in-95 pointer-events-none select-none',
            className
          )}
        >
          {content}
          <TooltipPrimitive.Arrow className="fill-popover" />
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );

  if (hasRootProvider) {
    return body;
  }

  return (
    <TooltipProvider delayDuration={delayDuration}>
      {body}
    </TooltipProvider>
  );
}

export { Tooltip, TooltipProvider };

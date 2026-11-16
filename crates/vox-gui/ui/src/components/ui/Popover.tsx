import React from 'react';
import { Popover as PopoverPrimitive } from 'radix-ui';
import { cn } from '../../lib/cn';

const PopoverRoot = PopoverPrimitive.Root;
const PopoverTrigger = PopoverPrimitive.Trigger;
const PopoverAnchor = PopoverPrimitive.Anchor;

const PopoverContent = React.forwardRef<
  React.ComponentRef<typeof PopoverPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof PopoverPrimitive.Content> & { portaled?: boolean }
>(({ className, align = 'center', sideOffset = 8, portaled = true, ...props }, ref) => {
  const node = (
    <PopoverPrimitive.Content
      ref={ref}
      data-slot="popover-content"
      align={align}
      sideOffset={sideOffset}
      collisionPadding={8}
      className={cn(
        'z-(--z-popover) min-w-[240px] rounded-lg border border-border-strong bg-popover p-1 text-popover-foreground shadow-[0_24px_60px_-20px_rgba(28,20,12,0.35)] backdrop-blur-xl outline-none',
        'data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95',
        className
      )}
      {...props}
    />
  );

  return portaled ? <PopoverPrimitive.Portal>{node}</PopoverPrimitive.Portal> : node;
});
PopoverContent.displayName = 'PopoverContent';

export interface PopoverProps {
  open: boolean;
  onOpenChange?: (open: boolean) => void;
  trigger?: React.ReactNode;
  children: React.ReactNode;
  align?: 'left' | 'right' | 'center';
  side?: 'top' | 'bottom' | 'left' | 'right';
  className?: string;
  modal?: boolean;
}

function Popover({
  open,
  onOpenChange,
  trigger,
  children,
  align = 'left',
  side = 'top',
  className = '',
  modal = false,
}: PopoverProps) {
  const radixAlign = align === 'right' ? 'end' : align === 'left' ? 'start' : 'center';

  if (trigger) {
    return (
      <PopoverRoot open={open} onOpenChange={onOpenChange} modal={modal}>
        <PopoverTrigger asChild data-slot="popover-trigger">
          {trigger}
        </PopoverTrigger>
        <PopoverContent align={radixAlign} side={side} className={className}>
          {children}
        </PopoverContent>
      </PopoverRoot>
    );
  }

  if (!open) return null;

  return (
    <PopoverRoot open={open} onOpenChange={onOpenChange} modal={modal}>
      <PopoverAnchor className="pointer-events-none block h-0 w-full" />
      <PopoverContent
        align={radixAlign}
        side={side}
        onOpenAutoFocus={(e) => e.preventDefault()}
        onCloseAutoFocus={(e) => e.preventDefault()}
        className={className}
      >
        {children}
      </PopoverContent>
    </PopoverRoot>
  );
}

export { Popover, PopoverRoot, PopoverTrigger, PopoverAnchor, PopoverContent };

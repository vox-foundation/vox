import React from 'react';
import { Select as SelectPrimitive } from 'radix-ui';
import { Check, ChevronDown, ChevronUp } from 'lucide-react';
import { cn } from '../../lib/cn';

const Select = SelectPrimitive.Root;
const SelectGroup = SelectPrimitive.Group;
const SelectValue = SelectPrimitive.Value;

const SelectTrigger = React.forwardRef<
  React.ComponentRef<typeof SelectPrimitive.Trigger>,
  React.ComponentPropsWithoutRef<typeof SelectPrimitive.Trigger>
>(({ className, children, ...props }, ref) => (
  <SelectPrimitive.Trigger
    ref={ref}
    type="button"
    data-slot="select-trigger"
    data-allow-ellipsis="true"
    className={cn(
      'flex h-8 w-full items-center justify-between gap-2 rounded-md border border-input bg-bg-surface px-2.5 py-1 text-xs text-text-primary shadow-xs transition-colors',
      'placeholder:text-text-muted hover:border-border-strong focus-visible:outline-solid focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:cursor-not-allowed disabled:opacity-50 [&>span]:line-clamp-1',
      className
    )}
    {...props}
  >
    {children}
    <SelectPrimitive.Icon asChild>
      <ChevronDown className="size-3.5 shrink-0 text-text-muted" strokeWidth={1.75} aria-hidden="true" />
    </SelectPrimitive.Icon>
  </SelectPrimitive.Trigger>
));
SelectTrigger.displayName = 'SelectTrigger';

const SelectContent = React.forwardRef<
  React.ComponentRef<typeof SelectPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof SelectPrimitive.Content>
>(({ className, children, position = 'popper', ...props }, ref) => (
  <SelectPrimitive.Portal>
    <SelectPrimitive.Content
      ref={ref}
      data-slot="select-content"
      position={position}
      collisionPadding={8}
      className={cn(
        'relative z-(--z-popover) max-h-72 min-w-[8rem] overflow-hidden rounded-lg border border-border-strong bg-popover text-popover-foreground shadow-lg',
        'data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95',
        position === 'popper' && 'data-[side=bottom]:translate-y-1 data-[side=top]:-translate-y-1',
        className
      )}
      {...props}
    >
      <SelectPrimitive.ScrollUpButton className="flex cursor-default items-center justify-center py-1 text-text-muted">
        <ChevronUp className="size-3.5" strokeWidth={1.75} aria-hidden="true" />
      </SelectPrimitive.ScrollUpButton>
      <SelectPrimitive.Viewport
        className={cn(
          'p-1',
          position === 'popper' && 'h-[var(--radix-select-trigger-height)] w-full min-w-[var(--radix-select-trigger-width)]'
        )}
      >
        {children}
      </SelectPrimitive.Viewport>
      <SelectPrimitive.ScrollDownButton className="flex cursor-default items-center justify-center py-1 text-text-muted">
        <ChevronDown className="size-3.5" strokeWidth={1.75} aria-hidden="true" />
      </SelectPrimitive.ScrollDownButton>
    </SelectPrimitive.Content>
  </SelectPrimitive.Portal>
));
SelectContent.displayName = 'SelectContent';

const SelectLabel = React.forwardRef<
  React.ComponentRef<typeof SelectPrimitive.Label>,
  React.ComponentPropsWithoutRef<typeof SelectPrimitive.Label>
>(({ className, ...props }, ref) => (
  <SelectPrimitive.Label
    ref={ref}
    data-slot="select-label"
    className={cn('px-2 py-1 text-[10px] font-semibold tracking-wider text-text-muted uppercase', className)}
    {...props}
  />
));
SelectLabel.displayName = 'SelectLabel';

const SelectItem = React.forwardRef<
  React.ComponentRef<typeof SelectPrimitive.Item>,
  React.ComponentPropsWithoutRef<typeof SelectPrimitive.Item>
>(({ className, children, ...props }, ref) => (
  <SelectPrimitive.Item
    ref={ref}
    data-slot="select-item"
    className={cn(
      'relative flex w-full cursor-default items-center rounded-sm py-1.5 pr-7 pl-2 text-xs text-popover-foreground outline-none select-none transition-colors',
      'focus:bg-overlay-hover focus:text-text-primary data-[disabled]:pointer-events-none data-[disabled]:opacity-50',
      className
    )}
    {...props}
  >
    <span className="absolute right-2 flex size-3.5 items-center justify-center">
      <SelectPrimitive.ItemIndicator>
        <Check className="size-3.5 text-primary" strokeWidth={2} aria-hidden="true" />
      </SelectPrimitive.ItemIndicator>
    </span>
    <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
  </SelectPrimitive.Item>
));
SelectItem.displayName = 'SelectItem';

export interface VoxSelectOption {
  readonly value: string;
  readonly label: React.ReactNode;
  readonly disabled?: boolean;
}

export interface VoxSelectProps {
  readonly value: string;
  readonly onValueChange: (next: string) => void;
  readonly options: readonly VoxSelectOption[];
  readonly placeholder?: string;
  readonly id?: string;
  readonly 'aria-label'?: string;
  readonly 'aria-labelledby'?: string;
  readonly 'data-testid'?: string;
  readonly title?: string;
  readonly disabled?: boolean;
  readonly size?: 'sm' | 'xs';
  readonly className?: string;
  readonly contentClassName?: string;
}

const EMPTY_VALUE_SENTINEL = '__vox_empty__';

function toWireSelectValue(v: string): string {
  return v === '' ? EMPTY_VALUE_SENTINEL : v;
}

function fromWireSelectValue(v: string): string {
  return v === EMPTY_VALUE_SENTINEL ? '' : v;
}

/**
 * Ergonomic Radix `<Select>` wrapper (`data-slot="select-trigger"`) that safely handles
 * empty-string option values (`""` <-> sentinel) and supports compact toolbar sizing.
 */
function VoxSelect({
  value,
  onValueChange,
  options,
  placeholder,
  id,
  'aria-label': ariaLabel,
  'aria-labelledby': ariaLabelledBy,
  'data-testid': dataTestId,
  title,
  disabled,
  size = 'sm',
  className,
  contentClassName,
}: VoxSelectProps) {
  return (
    <Select
      value={toWireSelectValue(value)}
      onValueChange={(next) => onValueChange(fromWireSelectValue(next))}
      disabled={disabled}
    >
      <SelectTrigger
        id={id}
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledBy}
        data-testid={dataTestId}
        title={title}
        className={cn(
          size === 'xs' && 'h-6.5 px-2 py-0.5 text-[11px]',
          className
        )}
      >
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent className={contentClassName}>
        {options.map((opt) => (
          <SelectItem
            key={toWireSelectValue(opt.value)}
            value={toWireSelectValue(opt.value)}
            disabled={opt.disabled}
          >
            {opt.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

export {
  Select,
  SelectGroup,
  SelectValue,
  SelectTrigger,
  SelectContent,
  SelectLabel,
  SelectItem,
  VoxSelect,
};

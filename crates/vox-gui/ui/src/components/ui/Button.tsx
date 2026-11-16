import React from 'react';
import { Slot as SlotPrimitive } from 'radix-ui';
import { cva, type VariantProps } from 'class-variance-authority';
import { Loader2 } from 'lucide-react';
import { cn } from '../../lib/cn';

const buttonVariants = cva(
  'inline-flex items-center justify-center font-medium tracking-wide transition-colors focus-visible:outline-solid focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brass disabled:pointer-events-none disabled:opacity-50',
  {
    variants: {
      variant: {
        primary: 'bg-brass text-primary-foreground hover:bg-brass-light active:bg-brass-dark',
        secondary: 'bg-overlay-subtle text-text-primary hover:bg-overlay-subtle active:bg-overlay-subtle',
        ghost: 'bg-transparent text-text-muted hover:text-text-primary hover:bg-overlay-subtle',
        outline: 'bg-transparent border border-border-subtle text-text-secondary hover:bg-overlay-subtle hover:text-text-primary',
        danger: 'bg-red-500 text-destructive-foreground hover:bg-red-600 active:bg-red-700',
        roman: 'bg-primary text-primary-foreground border border-border-strong shadow-xs hover:opacity-95 active:opacity-90',
        subtle: 'bg-muted text-muted-foreground hover:text-foreground hover:bg-accent',
        link: 'bg-transparent text-primary underline-offset-4 hover:underline',
      },
      size: {
        xs: 'px-2 py-0.5 text-[10px] h-6 rounded-sm whitespace-nowrap shrink-0',
        sm: 'px-2.5 py-0.5 text-[10.5px] h-7 rounded-md whitespace-nowrap shrink-0',
        md: 'px-3.5 py-1.5 text-[12.5px] h-9 rounded-lg whitespace-nowrap shrink-0',
        lg: 'px-4.5 py-2 text-[14.5px] h-11 rounded-xl whitespace-nowrap shrink-0',
        icon: 'size-8 p-0 flex items-center justify-center rounded-lg shrink-0',
      },
    },
    defaultVariants: {
      variant: 'secondary',
      size: 'md',
    },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  loading?: boolean;
  icon?: React.ReactNode;
  trailingIcon?: React.ReactNode;
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      variant = 'secondary',
      size = 'md',
      loading = false,
      icon,
      trailingIcon,
      asChild = false,
      className,
      type = 'button',
      children,
      disabled,
      ...props
    },
    ref
  ) => {
    const mergedClassName = cn(buttonVariants({ variant, size }), className);

    if (asChild) {
      return (
        <SlotPrimitive.Root
          ref={ref}
          data-slot="button"
          data-variant={variant ?? 'secondary'}
          className={mergedClassName}
          {...props}
        >
          {children}
        </SlotPrimitive.Root>
      );
    }

    return (
      <button
        ref={ref}
        type={type}
        data-slot="button"
        data-variant={variant ?? 'secondary'}
        disabled={loading || disabled}
        className={mergedClassName}
        {...props}
      >
        {loading ? (
          <Loader2 className="-ml-1 mr-2 size-3.5 shrink-0 animate-spin text-current" strokeWidth={1.75} aria-hidden="true" />
        ) : icon ? (
          <span className="mr-1.5 flex items-center">{icon}</span>
        ) : null}
        {children}
        {!loading && trailingIcon && <span className="ml-1.5 flex items-center">{trailingIcon}</span>}
      </button>
    );
  }
);

Button.displayName = 'Button';

export { Button, buttonVariants };

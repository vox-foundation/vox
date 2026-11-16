import React from 'react';
import { Slider as SliderPrimitive } from 'radix-ui';
import { cn } from '../../lib/cn';

export interface SliderProps
  extends Omit<React.ComponentPropsWithoutRef<typeof SliderPrimitive.Root>, 'aria-label'> {
  ariaLabel: string | string[];
}

function clampFinite(val: number, fallback: number): number {
  return Number.isFinite(val) ? val : fallback;
}

const Slider = React.forwardRef<React.ComponentRef<typeof SliderPrimitive.Root>, SliderProps>(
  ({ className, ariaLabel, min = 0, max = 100, step = 1, value, defaultValue, ...props }, ref) => {
    const safeMin = clampFinite(min, 0);
    const safeMax = Math.max(safeMin, clampFinite(max, 100));
    const safeStep = Math.max(Number.EPSILON, clampFinite(step, 1));
    const normalizedValue = value?.map((v) => Math.min(safeMax, Math.max(safeMin, clampFinite(v, safeMin))));
    const normalizedDefault = defaultValue?.map((v) =>
      Math.min(safeMax, Math.max(safeMin, clampFinite(v, safeMin)))
    );
    const thumbValues = normalizedValue ?? normalizedDefault ?? [safeMin];

    return (
      <SliderPrimitive.Root
        ref={ref}
        data-slot="slider"
        min={safeMin}
        max={safeMax}
        step={safeStep}
        value={normalizedValue}
        defaultValue={normalizedDefault}
        className={cn(
          'relative flex w-full touch-none items-center select-none data-[disabled]:opacity-50',
          className
        )}
        {...props}
      >
        <SliderPrimitive.Track
          data-slot="slider-track"
          className="relative h-1.5 w-full grow overflow-hidden rounded-full border border-border-subtle bg-bg-elevated"
        >
          <SliderPrimitive.Range
            data-slot="slider-range"
            className="absolute h-full bg-primary"
          />
        </SliderPrimitive.Track>
        {thumbValues.map((_, idx) => {
          const thumbLabel = Array.isArray(ariaLabel)
            ? (ariaLabel[idx] ?? ariaLabel[0] ?? 'Slider value')
            : ariaLabel;
          return (
            <SliderPrimitive.Thumb
              key={idx}
              data-slot="slider-thumb"
              aria-label={thumbLabel}
              className="block size-4 rounded-full border-2 border-primary bg-popover shadow-sm transition-colors hover:bg-bg-surface focus-visible:outline-solid focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:pointer-events-none"
            />
          );
        })}
      </SliderPrimitive.Root>
    );
  }
);
Slider.displayName = 'Slider';

export { Slider };

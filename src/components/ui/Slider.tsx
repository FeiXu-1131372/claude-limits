import { type InputHTMLAttributes, forwardRef, useId, useState } from 'react';

interface SliderProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type' | 'size'> {
  label: string;
  /** Display format for the value, e.g. (v) => `${v}m` */
  formatValue?: (value: number) => string;
  min?: number;
  max?: number;
  step?: number;
}

export const Slider = forwardRef<HTMLInputElement, SliderProps>(
  ({
    label,
    formatValue = (v) => String(v),
    min = 0,
    max = 100,
    step = 1,
    defaultValue,
    value: controlledValue,
    onChange,
    className = '',
    ...props
  }, ref) => {
    const id = useId();
    const [internalValue, setInternalValue] = useState(
      Number(controlledValue ?? defaultValue ?? min),
    );
    const value = controlledValue != null ? Number(controlledValue) : internalValue;

    const pct = ((value - min) / (max - min)) * 100;

    return (
      <div className={['flex flex-col gap-[var(--space-xs)]', className].join(' ')}>
        <div className="flex items-center justify-between">
          <label
            htmlFor={id}
            className="text-[var(--text-label)] font-[var(--weight-medium)] text-[var(--color-text-secondary)]"
          >
            {label}
          </label>
          <span className="mono text-[var(--text-label)] font-[var(--weight-semibold)] text-[var(--color-accent)]">
            {formatValue(value)}
          </span>
        </div>
        <div className="relative h-[20px] flex items-center">
          <input
            ref={ref}
            id={id}
            type="range"
            min={min}
            max={max}
            step={step}
            value={value}
            onChange={(e) => {
              setInternalValue(Number(e.target.value));
              onChange?.(e);
            }}
            className={[
              'w-full h-[5px] appearance-none rounded-[var(--radius-pill)]',
              'bg-[var(--color-track)] outline-none',
              'cursor-pointer',
              '[&::-webkit-slider-thumb]:appearance-none',
              '[&::-webkit-slider-thumb]:w-[16px] [&::-webkit-slider-thumb]:h-[16px]',
              '[&::-webkit-slider-thumb]:rounded-full',
              '[&::-webkit-slider-thumb]:bg-[var(--color-accent)]',
              '[&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[var(--color-bg-base)]',
              '[&::-webkit-slider-thumb]:cursor-pointer',
              '[&::-webkit-slider-thumb]:transition-transform [&::-webkit-slider-thumb]:duration-[var(--duration-fast)]',
              '[&::-webkit-slider-thumb]:ease-[var(--ease-spring)]',
              '[&::-webkit-slider-thumb]:hover:scale-110',
              '[&::-webkit-slider-thumb]:active:scale-95',
              '[&::-webkit-slider-thumb]:focus-visible:outline-2',
              '[&::-webkit-slider-thumb]:focus-visible:outline-[var(--color-border-focus)]',
            ].join(' ')}
            style={{
              background: `linear-gradient(to right, var(--color-accent) 0%, var(--color-accent) ${pct}%, var(--color-track) ${pct}%, var(--color-track) 100%)`,
            }}
            {...props}
          />
        </div>
      </div>
    );
  },
);

Slider.displayName = 'Slider';

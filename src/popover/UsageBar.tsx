import { forwardRef, type HTMLAttributes, useMemo } from 'react';

type ThresholdLevel = 'safe' | 'warn' | 'danger';

interface UsageBarProps extends Omit<HTMLAttributes<HTMLDivElement>, 'children'> {
  value: number;
  warnAt?: number;
  dangerAt?: number;
  size?: 'sm' | 'md';
  showLabel?: boolean;
  label?: string;
  timer?: string;
}

const heightMap = { sm: 'h-[var(--bar-height-sm)]', md: 'h-[var(--bar-height-md)]' };

function getLevel(v: number, warn: number, danger: number): ThresholdLevel {
  if (v >= danger) return 'danger';
  if (v >= warn) return 'warn';
  return 'safe';
}

const gradientMap: Record<ThresholdLevel, string> = {
  safe: 'from-[var(--color-safe)] to-[var(--color-accent)]',
  warn: 'from-[var(--color-accent)] to-[var(--color-warn)]',
  danger: 'from-[var(--color-warn)] to-[var(--color-danger)]',
};

const textColorMap: Record<ThresholdLevel, string> = {
  safe: 'text-[var(--color-text)]',
  warn: 'text-[var(--color-warn)]',
  danger: 'text-[var(--color-danger)]',
};

export const UsageBar = forwardRef<HTMLDivElement, UsageBarProps>(
  (
    {
      value,
      warnAt = 75,
      dangerAt = 90,
      size = 'md',
      showLabel = true,
      label,
      timer,
      className = '',
      ...props
    },
    ref,
  ) => {
    const clamped = Math.max(0, Math.min(100, value));
    const level = useMemo(() => getLevel(clamped, warnAt, dangerAt), [clamped, warnAt, dangerAt]);

    return (
      <div ref={ref} className={['flex flex-col gap-[6px]', className].join(' ')} {...props}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-[var(--space-sm)]">
            <span className="text-[var(--text-label)] font-[var(--weight-medium)] text-[var(--color-text-secondary)]">
              {label}
            </span>
            {timer && (
              <span className="mono text-[var(--text-micro)] text-[var(--color-text-muted)]">
                {timer}
              </span>
            )}
          </div>
          {showLabel && (
            <span
              className={[
                'mono text-[var(--text-title)] font-[var(--weight-semibold)] tabular-nums',
                textColorMap[level],
              ].join(' ')}
            >
              {Math.round(clamped)}%
            </span>
          )}
        </div>

        <div
          className={[
            'w-full rounded-[var(--radius-pill)] overflow-hidden',
            'bg-[var(--color-track)]',
            heightMap[size],
          ].join(' ')}
        >
          <div
            className={[
              'h-full rounded-[var(--radius-pill)] bg-gradient-to-r',
              gradientMap[level],
              'transition-[width] duration-[var(--duration-bar)] ease-[var(--ease-spring)]',
            ].join(' ')}
            style={{ width: `${clamped}%` }}
          />
        </div>
      </div>
    );
  },
);

UsageBar.displayName = 'UsageBar';

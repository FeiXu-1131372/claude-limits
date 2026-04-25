import { forwardRef, type HTMLAttributes, useMemo } from 'react';
import { ResetCountdown } from './ResetCountdown';
import type { Utilization } from '../lib/types';

type ThresholdLevel = 'safe' | 'warn' | 'danger';

interface UsageBarProps extends Omit<HTMLAttributes<HTMLDivElement>, 'children'> {
  label?: string;
  /** Pass a Utilization object from the API, OR use value/timer directly */
  data?: Utilization | null;
  /** Raw value override (used when data is null but we want to show a number) */
  value?: number;
  warnAt?: number;
  dangerAt?: number;
  size?: 'sm' | 'md';
  showLabel?: boolean;
  /** Raw timer string override */
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
      label,
      data,
      value: valueProp,
      warnAt = 75,
      dangerAt = 90,
      size = 'md',
      showLabel = true,
      timer: timerProp,
      className = '',
      ...props
    },
    ref,
  ) => {
    if (!data && valueProp === undefined) {
      return (
        <div className={['flex items-center justify-between py-2', className].join(' ')} {...props}>
          <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">{label}</span>
          <span className="mono text-[var(--text-label)] text-[var(--color-text-muted)] opacity-60">n/a</span>
        </div>
      );
    }

    const rawValue = data?.utilization ?? valueProp ?? 0;
    const clamped = Math.max(0, Math.min(100, rawValue));
    const level = useMemo(() => getLevel(clamped, warnAt, dangerAt), [clamped, warnAt, dangerAt]);

    return (
      <div ref={ref} className={['flex flex-col gap-[6px]', className].join(' ')} {...props}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-[var(--space-sm)]">
            <span className="text-[var(--text-label)] font-[var(--weight-medium)] text-[var(--color-text-secondary)]">
              {label}
            </span>
            {data?.resets_at && (
              <ResetCountdown resetsAt={data.resets_at} />
            )}
            {timerProp && !data?.resets_at && (
              <span className="mono text-[var(--text-micro)] text-[var(--color-text-muted)]">
                {timerProp}
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

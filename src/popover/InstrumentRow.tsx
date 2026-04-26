/**
 * InstrumentColumn / InstrumentRow — the typographic hero of the popover.
 *
 * The big idea: numbers ARE the design. We render the percentage as a large
 * mono digit with hairline progress underneath, in two formats:
 *
 *   - InstrumentColumn (for 5h / 7d primary): big 56px digits, full-column
 *     progress hairline, reset countdown underneath.
 *   - InstrumentRow (for sub-rows like Opus / Sonnet / Pay-as-you-go): label
 *     left-aligned, smaller mono digit right-aligned, hairline below.
 *
 * Color appears in the meter only — never on the digit itself. That keeps the
 * focal point typographic, with status read at a glance from the meter color.
 */

import { ResetCountdown } from './ResetCountdown';
import type { Utilization } from '../lib/types';

type Level = 'safe' | 'warn' | 'danger' | 'idle';

function levelOf(value: number | null, warn: number, danger: number): Level {
  if (value == null) return 'idle';
  if (value >= danger) return 'danger';
  if (value >= warn) return 'warn';
  return 'safe';
}

const meterColor: Record<Level, string> = {
  idle: 'var(--color-rule-strong)',
  safe: 'var(--color-safe)',
  warn: 'var(--color-warn)',
  danger: 'var(--color-danger)',
};

/* ─── Primary instrument: big stacked column ─── */

export function InstrumentColumn({
  label,
  data,
  warnAt,
  dangerAt,
}: {
  label: string;
  data: Utilization | null;
  warnAt: number;
  dangerAt: number;
}) {
  const value = data?.utilization ?? null;
  const level = levelOf(value, warnAt, dangerAt);
  const clamped = value == null ? 0 : Math.max(0, Math.min(100, value));

  return (
    <div className="flex flex-col gap-[10px]">
      {/* Eyebrow label */}
      <span className="text-[var(--text-micro)] font-[var(--weight-medium)] tracking-[var(--tracking-label)] uppercase text-[var(--color-text-muted)]">
        {label}
      </span>

      {/* Hero number */}
      <div className="flex items-baseline gap-[2px]">
        {value == null ? (
          <span
            className="text-[var(--text-display)] text-[var(--color-text-muted)] leading-[var(--leading-display)]"
            style={{ fontFamily: 'var(--font-mono)' }}
          >
            —
          </span>
        ) : (
          <>
            <HeroNumber value={Math.round(value)} />
            <span
              className="text-[var(--text-pct)] font-[var(--weight-medium)] text-[var(--color-text-secondary)]"
              style={{
                fontFamily: 'var(--font-mono)',
                lineHeight: 'var(--leading-hero)',
                letterSpacing: 'var(--tracking-hero)',
              }}
            >
              %
            </span>
          </>
        )}
      </div>

      {/* Hairline meter */}
      <Meter value={clamped} level={level} />

      {/* Caption: reset countdown */}
      <span className="h-[14px] text-[var(--text-micro)] text-[var(--color-text-muted)]">
        {data?.resets_at ? <ResetCountdown resetsAt={data.resets_at} /> : ' '}
      </span>
    </div>
  );
}

/* ─── Secondary instrument: inline row ─── */

export function InstrumentRow({
  label,
  caption,
  value,
  data,
  warnAt,
  dangerAt,
}: {
  label: string;
  caption?: string;
  value?: number;
  data?: Utilization | null;
  warnAt: number;
  dangerAt: number;
}) {
  const v = value ?? data?.utilization ?? null;
  const level = levelOf(v, warnAt, dangerAt);
  const clamped = v == null ? 0 : Math.max(0, Math.min(100, v));

  return (
    <div className="flex flex-col gap-[6px]">
      <div className="flex items-baseline justify-between gap-[var(--space-sm)] min-w-0">
        <div className="flex items-baseline gap-[var(--space-xs)] min-w-0">
          <span className="text-[var(--text-label)] font-[var(--weight-medium)] text-[var(--color-text-secondary)] truncate">
            {label}
          </span>
          {(data?.resets_at || caption) && (
            <span className="text-[var(--text-micro)] text-[var(--color-text-muted)] truncate">
              {data?.resets_at ? <ResetCountdown resetsAt={data.resets_at} /> : caption}
            </span>
          )}
        </div>
        <span
          className="text-[var(--text-pct)] font-[var(--weight-medium)] tabular-nums leading-none shrink-0"
          style={{
            fontFamily: 'var(--font-mono)',
            color: v == null ? 'var(--color-text-muted)' : 'var(--color-text)',
            letterSpacing: '-0.02em',
          }}
        >
          {v == null ? '—' : `${Math.round(v)}%`}
        </span>
      </div>
      <Meter value={clamped} level={level} small />
    </div>
  );
}

/* ─── Hairline meter ─── */

function Meter({ value, level, small }: { value: number; level: Level; small?: boolean }) {
  return (
    <div
      className="relative w-full overflow-hidden rounded-full"
      style={{
        height: small ? '2px' : '3px',
        background: 'var(--color-track)',
      }}
    >
      <div
        className="h-full rounded-full transition-[width,background] duration-[var(--duration-bar)] ease-[var(--ease-out)]"
        style={{
          width: `${value}%`,
          background: meterColor[level],
        }}
      />
    </div>
  );
}

/* ─── Hero number with subtle weight on the digit pair ─── */

function HeroNumber({ value }: { value: number }) {
  // Render so leading zero behavior is sensible: 7 → "7", 11 → "11", 100 → "100".
  return (
    <span
      className="font-[var(--weight-medium)] tabular-nums text-[var(--color-text)] inline-block"
      style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 'var(--text-hero)',
        lineHeight: 'var(--leading-hero)',
        letterSpacing: 'var(--tracking-hero)',
        fontFeatureSettings: '"tnum", "ss01"',
      }}
    >
      {value}
    </span>
  );
}

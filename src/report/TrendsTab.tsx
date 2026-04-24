import { useMemo, useState } from 'react';
import { Card } from '../components/ui/Card';
import { EmptyState } from '../components/ui/EmptyState';
import type { DailyBucket } from '../lib/types';
import { formatTokens } from '../lib/format';
import { IconTrends } from '../lib/icons';

/* Placeholder data — 30 days */
const PLACEHOLDER: DailyBucket[] = Array.from({ length: 30 }, (_, i) => {
  const d = new Date();
  d.setDate(d.getDate() - (29 - i));
  return {
    date: d.toISOString().slice(0, 10),
    five_hour_pct: 20 + Math.floor(Math.random() * 70),
    seven_day_pct: 30 + Math.floor(Math.random() * 50),
    tokens: 100_000 + Math.floor(Math.random() * 900_000),
    cost_usd: 0.5 + Math.random() * 3,
  };
});

type Range = '7d' | '30d';

export function TrendsTab() {
  const [range, setRange] = useState<Range>('30d');
  const data = PLACEHOLDER;

  const visibleData = useMemo(() => {
    const days = range === '7d' ? 7 : 30;
    return data.slice(-days);
  }, [data, range]);

  if (data.length === 0) {
    return (
      <EmptyState
        icon={<IconTrends size={32} />}
        title="No trend data"
        description="Trends will appear after a few days of usage."
      />
    );
  }

  const maxValue = Math.max(...visibleData.map((d) => d.tokens), 1);
  const chartHeight = 160;
  const barWidth = 100 / visibleData.length;

  return (
    <div className="flex flex-col gap-[var(--space-md)]">
      {/* Range selector */}
      <div className="flex gap-[var(--space-2xs)] bg-[var(--color-track)] rounded-[var(--radius-sm)] p-[2px] w-fit">
        {(['7d', '30d'] as Range[]).map((r) => (
          <button
            key={r}
            onClick={() => setRange(r)}
            className={[
              'px-[var(--space-sm)] py-[var(--space-2xs)]',
              'text-[var(--text-label)] font-[var(--weight-medium)]',
              'rounded-[var(--radius-sm)]',
              'transition-[background,color] duration-[var(--duration-fast)]',
              range === r
                ? 'bg-[var(--color-bg-card)] text-[var(--color-text)]'
                : 'text-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)]',
            ].join(' ')}
          >
            {r}
          </button>
        ))}
      </div>

      {/* Chart */}
      <Card className="p-[var(--space-md)]">
        <div className="flex items-end gap-[2px]" style={{ height: chartHeight }}>
          {visibleData.map((day, i) => {
            const heightPct = (day.tokens / maxValue) * 100;
            const fivePct = day.five_hour_pct;
            const isWarn = fivePct >= 75 && fivePct < 90;
            const isDanger = fivePct >= 90;

            return (
              <div
                key={day.date}
                className="flex-1 flex flex-col justify-end group relative"
                style={{ height: '100%' }}
              >
                <div
                  className={[
                    'w-full rounded-t-[2px] transition-[height] duration-[var(--duration-normal)]',
                    'bg-gradient-to-t',
                    isDanger
                      ? 'from-[var(--color-warn)] to-[var(--color-danger)]'
                      : isWarn
                        ? 'from-[var(--color-accent)] to-[var(--color-warn)]'
                        : 'from-[var(--color-safe)] to-[var(--color-accent)]',
                    'opacity-80 group-hover:opacity-100',
                  ].join(' ')}
                  style={{ height: `${heightPct}%` }}
                />
                {/* Tooltip */}
                <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-[var(--space-xs)] hidden group-hover:block z-10">
                  <div className="bg-[var(--color-bg-elevated)] border border-[var(--color-border)] rounded-[var(--radius-sm)] px-[var(--space-sm)] py-[var(--space-xs)] whitespace-nowrap">
                    <div className="text-[var(--text-micro)] text-[var(--color-text-muted)]">
                      {new Date(day.date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}
                    </div>
                    <div className="mono text-[var(--text-label)] text-[var(--color-text)]">
                      {formatTokens(day.tokens)}
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        {/* X-axis labels */}
        <div className="flex justify-between mt-[var(--space-xs)]">
          {visibleData
            .filter((_, i) => i % (range === '7d' ? 1 : 5) === 0)
            .map((day) => (
              <span key={day.date} className="text-[var(--text-micro)] text-[var(--color-text-muted)] mono">
                {new Date(day.date).toLocaleDateString('en-US', { day: 'numeric' })}
              </span>
            ))}
        </div>
      </Card>

      {/* Inline summary — no hero metric cards */}
      <div className="flex items-center gap-[var(--space-md)] px-[var(--space-2xs)]">
        <span className="mono text-[var(--text-label)] text-[var(--color-text-secondary)]">
          Avg {formatTokens(visibleData.reduce((s, d) => s + d.tokens, 0) / visibleData.length)}
        </span>
        <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">·</span>
        <span className="mono text-[var(--text-label)] text-[var(--color-text-secondary)]">
          Peak {formatTokens(Math.max(...visibleData.map((d) => d.tokens)))}
        </span>
        <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">·</span>
        <span className="mono text-[var(--text-label)] text-[var(--color-text-secondary)]">
          ${visibleData.reduce((s, d) => s + d.cost_usd, 0).toFixed(2)} total
        </span>
      </div>
    </div>
  );
}

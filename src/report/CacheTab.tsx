import { Card } from '../components/ui/Card';
import { EmptyState } from '../components/ui/EmptyState';
import type { CacheStats } from '../lib/types';
import { formatTokens } from '../lib/format';
import { IconCache } from '../lib/icons';

const PLACEHOLDER: CacheStats = {
  cache_read_tokens: 14_200_000,
  cache_write_tokens: 3_800_000,
  total_input_tokens: 22_400_000,
  estimated_savings_usd: 42.6,
  hit_rate_pct: 63,
};

export function CacheTab() {
  const data = PLACEHOLDER;

  if (data.total_input_tokens === 0) {
    return (
      <EmptyState
        icon={<IconCache size={32} />}
        title="No cache data"
        description="Cache statistics will appear as you use Claude with prompt caching."
      />
    );
  }

  const hitRate = data.hit_rate_pct;
  const circumference = 2 * Math.PI * 50;
  const strokeLength = (hitRate / 100) * circumference;

  return (
    <div className="flex flex-col gap-[var(--space-lg)]">
      {/* Hero: cache hit rate ring */}
      <div className="flex items-center justify-center py-[var(--space-lg)]">
        <div className="relative">
          <svg width="160" height="160" viewBox="0 0 160 160">
            <circle
              cx="80"
              cy="80"
              r="50"
              fill="none"
              stroke="var(--color-track)"
              strokeWidth="12"
            />
            <circle
              cx="80"
              cy="80"
              r="50"
              fill="none"
              stroke="var(--color-accent)"
              strokeWidth="12"
              strokeLinecap="round"
              strokeDasharray={`${strokeLength} ${circumference - strokeLength}`}
              transform="rotate(-90 80 80)"
              className="transition-[stroke-dasharray] duration-[var(--duration-slow)] ease-[var(--ease-spring)]"
            />
          </svg>
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <span className="mono text-[28px] font-[var(--weight-semibold)] text-[var(--color-accent)] leading-[var(--leading-display)]">
              {Math.round(hitRate)}%
            </span>
            <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">cache hit rate</span>
          </div>
        </div>
      </div>

      {/* Stats grid */}
      <div className="grid grid-cols-2 gap-[var(--space-sm)]">
        <Card className="p-[var(--space-md)] flex flex-col gap-[4px]">
          <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">Cache reads</span>
          <span className="mono text-[var(--text-body)] font-[var(--weight-semibold)] text-[var(--color-text)]">
            {formatTokens(data.cache_read_tokens)}
          </span>
        </Card>
        <Card className="p-[var(--space-md)] flex flex-col gap-[4px]">
          <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">Cache writes</span>
          <span className="mono text-[var(--text-body)] font-[var(--weight-semibold)] text-[var(--color-text)]">
            {formatTokens(data.cache_write_tokens)}
          </span>
        </Card>
        <Card className="p-[var(--space-md)] flex flex-col gap-[4px]">
          <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">Total input</span>
          <span className="mono text-[var(--text-body)] font-[var(--weight-semibold)] text-[var(--color-text)]">
            {formatTokens(data.total_input_tokens)}
          </span>
        </Card>
        <Card className="p-[var(--space-md)] flex flex-col gap-[4px]">
          <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">Estimated savings</span>
          <span className="mono text-[var(--text-body)] font-[var(--weight-semibold)] text-[var(--color-safe)]">
            ${data.estimated_savings_usd.toFixed(2)}
          </span>
        </Card>
      </div>

      {/* Breakdown bar */}
      <Card className="p-[var(--space-md)]">
        <div className="flex flex-col gap-[var(--space-sm)]">
          <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">Input token breakdown</span>
          <div className="flex h-[10px] rounded-[var(--radius-pill)] overflow-hidden gap-[1px]">
            <div
              className="h-full bg-[var(--color-accent)] rounded-l-[var(--radius-pill)]"
              style={{ width: `${hitRate}%` }}
            />
            <div
              className="h-full bg-[var(--color-warn)]"
              style={{ width: `${(data.cache_write_tokens / data.total_input_tokens) * 100}%` }}
            />
            <div
              className="h-full bg-[var(--color-track)] rounded-r-[var(--radius-pill)]"
              style={{ width: `${100 - hitRate - (data.cache_write_tokens / data.total_input_tokens) * 100}%` }}
            />
          </div>
          <div className="flex gap-[var(--space-md)]">
            <div className="flex items-center gap-[var(--space-2xs)]">
              <div className="w-[8px] h-[8px] rounded-full bg-[var(--color-accent)]" />
              <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">Cached</span>
            </div>
            <div className="flex items-center gap-[var(--space-2xs)]">
              <div className="w-[8px] h-[8px] rounded-full bg-[var(--color-warn)]" />
              <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">Write</span>
            </div>
            <div className="flex items-center gap-[var(--space-2xs)]">
              <div className="w-[8px] h-[8px] rounded-full bg-[var(--color-track)]" />
              <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">New</span>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}

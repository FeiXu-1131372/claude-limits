import { useMemo } from 'react';
import { Card } from '../components/ui/Card';
import { Badge } from '../components/ui/Badge';
import { EmptyState } from '../components/ui/EmptyState';
import type { ModelStats } from '../lib/types';
import { formatTokens, formatCost } from '../lib/format';
import { IconChart } from '../lib/icons';

const MODEL_VARIANT: Record<string, 'opus' | 'sonnet' | 'haiku' | 'default'> = {
  opus: 'opus',
  sonnet: 'sonnet',
  haiku: 'haiku',
};

/* Placeholder */
const PLACEHOLDER: ModelStats[] = [
  { model: 'claude-opus-4-20250514', input_tokens: 8_000_000, output_tokens: 4_400_000, cache_read_tokens: 2_000_000, cache_creation_tokens: 1_000_000, cost_usd: 18.6 },
  { model: 'claude-sonnet-4-20250514', input_tokens: 5_200_000, output_tokens: 3_000_000, cache_read_tokens: 1_500_000, cache_creation_tokens: 800_000, cost_usd: 4.1 },
  { model: 'claude-haiku-4-5-20251001', input_tokens: 1_000_000, output_tokens: 800_000, cache_read_tokens: 400_000, cache_creation_tokens: 200_000, cost_usd: 0.36 },
];

function modelKey(name: string): string {
  const lower = name.toLowerCase();
  if (lower.includes('opus')) return 'opus';
  if (lower.includes('sonnet')) return 'sonnet';
  if (lower.includes('haiku')) return 'haiku';
  return 'default';
}

export function ModelsTab() {
  const data = PLACEHOLDER;

  const totalTokens = useMemo(
    () => data.reduce((s, m) => s + m.input_tokens + m.output_tokens, 0),
    [data],
  );
  const totalCost = useMemo(
    () => data.reduce((s, m) => s + m.cost_usd, 0),
    [data],
  );

  const segments = useMemo(() => {
    return data.map((m) => ({
      ...m,
      key: modelKey(m.model),
      total: m.input_tokens + m.output_tokens,
      pct: totalTokens > 0 ? ((m.input_tokens + m.output_tokens) / totalTokens) * 100 : 0,
    }));
  }, [data, totalTokens]);

  if (data.length === 0) {
    return (
      <EmptyState
        icon={<IconChart size={32} />}
        title="No model data"
        description="Model breakdown will appear after your first sessions."
      />
    );
  }

  /* Donut chart via SVG */
  const radius = 50;
  const circumference = 2 * Math.PI * radius;
  let accumulatedOffset = 0;

  return (
    <div className="flex flex-col gap-[var(--space-lg)]">
      {/* Donut chart */}
      <div className="flex items-center justify-center py-[var(--space-lg)]">
        <div className="relative">
          <svg width="140" height="140" viewBox="0 0 140 140">
            {segments.map((seg) => {
              const strokeLength = (seg.pct / 100) * circumference;
              const offset = accumulatedOffset;
              accumulatedOffset += strokeLength;

              const colors: Record<string, string> = {
                opus: 'var(--color-accent)',
                sonnet: 'var(--color-warn)',
                haiku: 'var(--color-safe)',
              };

              return (
                <circle
                  key={seg.model}
                  cx="70"
                  cy="70"
                  r={radius}
                  fill="none"
                  stroke={colors[seg.key] ?? 'var(--color-text-muted)'}
                  strokeWidth="14"
                  strokeDasharray={`${strokeLength} ${circumference - strokeLength}`}
                  strokeDashoffset={-offset}
                  strokeLinecap="round"
                  transform="rotate(-90 70 70)"
                  className="transition-[stroke-dasharray,stroke-dashoffset] duration-[var(--duration-slow)] ease-[var(--ease-spring)]"
                />
              );
            })}
          </svg>
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <span className="mono text-[var(--text-title)] font-[var(--weight-semibold)] text-[var(--color-text)]">
              {formatTokens(totalTokens)}
            </span>
            <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">tokens</span>
          </div>
        </div>
      </div>

      {/* Model list */}
      <div className="flex flex-col gap-[var(--space-sm)]">
        {segments.map((seg) => (
          <Card key={seg.model} className="p-[var(--space-sm)]">
            <div className="flex items-center gap-[var(--space-sm)]">
              <Badge variant={MODEL_VARIANT[seg.key] ?? 'default'}>
                {seg.key}
              </Badge>
              <div className="flex-1">
                <div className="w-full h-[6px] rounded-[var(--radius-pill)] bg-[var(--color-track)] overflow-hidden">
                  <div
                    className="h-full rounded-[var(--radius-pill)] transition-[width] duration-[var(--duration-bar)] ease-[var(--ease-spring)]"
                    style={{
                      width: `${seg.pct}%`,
                      background:
                        seg.key === 'opus'
                          ? 'var(--color-accent)'
                          : seg.key === 'sonnet'
                            ? 'var(--color-warn)'
                            : 'var(--color-safe)',
                    }}
                  />
                </div>
              </div>
              <div className="flex items-center gap-[var(--space-md)] shrink-0">
                <span className="mono text-[var(--text-label)] text-[var(--color-text-secondary)] tabular-nums min-w-[52px] text-right">
                  {seg.pct.toFixed(0)}%
                </span>
                <span className="mono text-[var(--text-label)] text-[var(--color-text-muted)] tabular-nums min-w-[48px] text-right">
                  {formatCost(seg.cost_usd)}
                </span>
              </div>
            </div>
          </Card>
        ))}
      </div>

      {/* Total */}
      <div className="flex items-center justify-between px-[var(--space-2xs)]">
        <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">Total</span>
        <span className="mono text-[var(--text-body)] font-[var(--weight-semibold)] text-[var(--color-text)]">
          {formatCost(totalCost)}
        </span>
      </div>
    </div>
  );
}

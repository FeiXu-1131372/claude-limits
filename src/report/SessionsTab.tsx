import { useEffect, useMemo, useState } from 'react';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { EmptyState } from '../components/ui/EmptyState';
import { formatTokens, formatCost } from '../lib/format';
import { ChevronDown, ChevronRight, IconSessions } from '../lib/icons';
import { ipc } from '../lib/ipc';
import { useTabData } from '../lib/useTabData';
import { useAppStore } from '../lib/store';
import type { PricingEntry, SessionEvent } from '../lib/types';
import { costPerCategory, lookupPricing } from '../lib/pricing';

const MODEL_BADGE: Record<string, 'opus' | 'sonnet' | 'haiku' | 'default'> = {
  opus: 'opus',
  sonnet: 'sonnet',
  haiku: 'haiku',
};

function modelKey(name: string): string {
  const lower = name.toLowerCase();
  if (lower.includes('opus')) return 'opus';
  if (lower.includes('sonnet')) return 'sonnet';
  if (lower.includes('haiku')) return 'haiku';
  return 'default';
}

function formatTime(iso: string): string {
  const d = new Date(iso);
  const now = new Date();
  const isToday = d.toDateString() === now.toDateString();
  const time = d.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
  if (isToday) return time;
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }) + ' ' + time;
}

interface AggregatedSession {
  id: string;
  project: string;
  latest_ts: string;
  turn_count: number;
  /** Headline tokens shown on the collapsed row: input + output only.
   * This represents the "new content" of the session, which is what
   * users intuitively mean by "size of the session." Cache totals
   * (which can be 10–100x larger but mostly reuse) are surfaced in the
   * expandable breakdown. */
  headline_tokens: number;
  total_cost_usd: number;
  /** Model with the most output tokens — surfaced via the row's badge. */
  dominant_model: string;
  /** Per-category token totals for the expandable breakdown. */
  breakdown: {
    input: number;
    output: number;
    cache_read: number;
    cache_write_5m: number;
    cache_write_1h: number;
  };
  /** Per-category cost contributions, summed over all turns. Mirrors the
   * backend cost_for logic so totals match what's billed. */
  cost_breakdown: {
    input: number;
    output: number;
    cache_read: number;
    cache_write_5m: number;
    cache_write_1h: number;
  };
}

/**
 * Group raw assistant-message events into one row per Claude Code session.
 * Each unique `source_file` (a JSONL path under ~/.claude/projects/) is one
 * session; the lines inside it are the conversation turns. Without this,
 * "5447 sessions" actually meant 5447 individual API calls.
 */
function aggregateSessions(
  events: SessionEvent[],
  pricing: PricingEntry[] | null,
): AggregatedSession[] {
  const byFile = new Map<string, AggregatedSession & { _modelTokens: Map<string, number> }>();

  for (const e of events) {
    const id = e.source_file;
    let agg = byFile.get(id);
    if (!agg) {
      agg = {
        id,
        project: e.project,
        latest_ts: e.ts,
        turn_count: 0,
        headline_tokens: 0,
        total_cost_usd: 0,
        dominant_model: e.model,
        breakdown: {
          input: 0,
          output: 0,
          cache_read: 0,
          cache_write_5m: 0,
          cache_write_1h: 0,
        },
        cost_breakdown: {
          input: 0,
          output: 0,
          cache_read: 0,
          cache_write_5m: 0,
          cache_write_1h: 0,
        },
        _modelTokens: new Map(),
      };
      byFile.set(id, agg);
    }
    agg.turn_count += 1;
    agg.headline_tokens += e.input_tokens + e.output_tokens;
    agg.breakdown.input += e.input_tokens;
    agg.breakdown.output += e.output_tokens;
    agg.breakdown.cache_read += e.cache_read_tokens;
    agg.breakdown.cache_write_5m += e.cache_creation_5m_tokens;
    agg.breakdown.cache_write_1h += e.cache_creation_1h_tokens;
    agg.total_cost_usd += e.cost_usd;
    if (e.ts > agg.latest_ts) agg.latest_ts = e.ts;
    agg._modelTokens.set(
      e.model,
      (agg._modelTokens.get(e.model) ?? 0) + e.output_tokens,
    );

    // Per-category cost is computed per-turn so the Sonnet 1M-context
    // tier (which switches based on the call's own context size, not the
    // session total) is applied correctly.
    if (pricing) {
      const entry = lookupPricing(pricing, e.model);
      if (entry) {
        const c = costPerCategory(entry, {
          input: e.input_tokens,
          output: e.output_tokens,
          cache_read: e.cache_read_tokens,
          cache_5m: e.cache_creation_5m_tokens,
          cache_1h: e.cache_creation_1h_tokens,
        });
        agg.cost_breakdown.input += c.input;
        agg.cost_breakdown.output += c.output;
        agg.cost_breakdown.cache_read += c.cache_read;
        agg.cost_breakdown.cache_write_5m += c.cache_5m;
        agg.cost_breakdown.cache_write_1h += c.cache_1h;
      }
    }
  }

  const result: AggregatedSession[] = [];
  for (const agg of byFile.values()) {
    let bestModel = agg.dominant_model;
    let bestTokens = -1;
    for (const [model, tokens] of agg._modelTokens) {
      if (tokens > bestTokens) {
        bestTokens = tokens;
        bestModel = model;
      }
    }
    agg.dominant_model = bestModel;
    const { _modelTokens, ...rest } = agg;
    void _modelTokens;
    result.push(rest);
  }

  result.sort((a, b) => (a.latest_ts < b.latest_ts ? 1 : -1));
  return result;
}

const BREAKDOWN_ROWS: Array<{
  key: keyof AggregatedSession['breakdown'];
  costKey: keyof AggregatedSession['cost_breakdown'];
  label: string;
}> = [
  { key: 'input', costKey: 'input', label: 'Input' },
  { key: 'output', costKey: 'output', label: 'Output' },
  { key: 'cache_read', costKey: 'cache_read', label: 'Cache read' },
  { key: 'cache_write_5m', costKey: 'cache_write_5m', label: 'Cache write (5m)' },
  { key: 'cache_write_1h', costKey: 'cache_write_1h', label: 'Cache write (1h)' },
];

function BreakdownTable({
  breakdown,
  costBreakdown,
}: {
  breakdown: AggregatedSession['breakdown'];
  costBreakdown: AggregatedSession['cost_breakdown'];
}) {
  // Hide rows with zero tokens — keeps the table tight, since most
  // sessions don't use the 1h cache tier at all.
  const rows = BREAKDOWN_ROWS.filter((r) => breakdown[r.key] > 0);
  return (
    <div
      className={[
        'mx-[var(--space-sm)] mb-[var(--space-sm)]',
        'rounded-[var(--radius-md)] border border-[var(--color-border-subtle)]',
        'bg-[var(--color-bg-card)]',
      ].join(' ')}
    >
      {rows.map((r, i) => (
        <div
          key={r.key}
          className={[
            'flex items-center justify-between gap-[var(--space-md)] px-[var(--space-md)] py-[var(--space-xs)]',
            i < rows.length - 1 ? 'border-b border-[var(--color-border-subtle)]' : '',
          ].join(' ')}
        >
          <span className="flex-1 text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
            {r.label}
          </span>
          <span className="mono text-[length:var(--text-label)] text-[color:var(--color-text-secondary)] tabular-nums min-w-[64px] text-right">
            {formatTokens(breakdown[r.key])}
          </span>
          <span className="mono text-[length:var(--text-label)] text-[color:var(--color-text-muted)] tabular-nums min-w-[56px] text-right">
            {formatCost(costBreakdown[r.costKey])}
          </span>
        </div>
      ))}
    </div>
  );
}

export function SessionsTab() {
  const version = useAppStore((s) => s.sessionDataVersion);
  const { data: events, error, loading, reload } = useTabData(
    () => ipc.getSessionHistory(7),
    [version],
  );
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [pricing, setPricing] = useState<PricingEntry[] | null>(null);

  // Pricing rates are static for the lifetime of the process; fetch once.
  useEffect(() => {
    let cancelled = false;
    ipc.getPricing().then((p) => {
      if (!cancelled) setPricing(p);
    }).catch(() => {});
    return () => { cancelled = true; };
  }, []);

  const sessions = useMemo(
    () => aggregateSessions(events ?? [], pricing),
    [events, pricing],
  );
  const totalCost = useMemo(
    () => sessions.reduce((sum, s) => sum + s.total_cost_usd, 0),
    [sessions],
  );

  if (error) {
    return (
      <EmptyState
        icon={<IconSessions size={32} />}
        title="Couldn't load sessions"
        description={error}
        action={<Button variant="ghost" size="sm" onClick={reload}>Retry</Button>}
      />
    );
  }
  if (loading || !events) {
    return <p className="text-[color:var(--color-text-muted)]">Loading…</p>;
  }

  if (sessions.length === 0) {
    return (
      <EmptyState
        icon={<IconSessions size={32} />}
        title="No sessions yet"
        description="Sessions will appear here as you use Claude Code."
      />
    );
  }

  return (
    <div className="flex flex-col gap-[var(--space-sm)]">
      <div className="flex items-center justify-between px-[var(--space-2xs)]">
        <span className="text-[length:var(--text-label)] text-[color:var(--color-text-muted)]">
          {sessions.length} {sessions.length === 1 ? 'session' : 'sessions'}
        </span>
        <span className="mono text-[length:var(--text-label)] text-[color:var(--color-text-secondary)]">
          {formatCost(totalCost)}
        </span>
      </div>

      <div className="flex flex-col">
        {sessions.slice(0, 100).map((session) => {
          const key = modelKey(session.dominant_model);
          const isOpen = expandedId === session.id;
          const Chevron = isOpen ? ChevronDown : ChevronRight;
          return (
            <div key={session.id} className="border-b border-[var(--color-border-subtle)]">
              <button
                type="button"
                onClick={() => setExpandedId(isOpen ? null : session.id)}
                className={[
                  'w-full flex items-center gap-[var(--space-sm)]',
                  'px-[var(--space-sm)] py-[var(--space-sm)]',
                  'text-left',
                  'transition-[background] duration-[var(--duration-fast)]',
                  'hover:bg-[var(--color-bg-card)]',
                  isOpen ? 'bg-[var(--color-bg-card)]' : '',
                ].join(' ')}
                aria-expanded={isOpen}
              >
                <Chevron
                  size={14}
                  className="shrink-0 text-[color:var(--color-text-muted)]"
                />
                <div className="flex flex-col min-w-0 flex-1">
                  <div className="flex items-center gap-[var(--space-sm)]">
                    <span className="text-[length:var(--text-body)] text-[color:var(--color-text)] truncate">
                      {session.project}
                    </span>
                    <Badge variant={MODEL_BADGE[key] ?? 'default'}>
                      {key}
                    </Badge>
                  </div>
                  <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
                    {formatTime(session.latest_ts)} · {session.turn_count} {session.turn_count === 1 ? 'turn' : 'turns'}
                  </span>
                </div>

                <div className="flex items-center gap-[var(--space-md)] shrink-0">
                  <span className="mono text-[length:var(--text-label)] text-[color:var(--color-text-secondary)] tabular-nums">
                    {formatTokens(session.headline_tokens)}
                  </span>
                  <span className="mono text-[length:var(--text-label)] text-[color:var(--color-text-muted)] tabular-nums min-w-[48px] text-right">
                    {formatCost(session.total_cost_usd)}
                  </span>
                </div>
              </button>
              {isOpen && (
                <BreakdownTable
                  breakdown={session.breakdown}
                  costBreakdown={session.cost_breakdown}
                />
              )}
            </div>
          );
        })}
        {sessions.length > 100 && (
          <div className="py-[var(--space-md)] text-center text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
            Showing latest 100 sessions.
          </div>
        )}
      </div>
    </div>
  );
}

import { useMemo } from 'react';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { EmptyState } from '../components/ui/EmptyState';
import { formatTokens, formatCost } from '../lib/format';
import { IconSessions } from '../lib/icons';
import { ipc } from '../lib/ipc';
import { useTabData } from '../lib/useTabData';
import { useAppStore } from '../lib/store';
import type { SessionEvent } from '../lib/types';

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
  total_tokens: number;
  total_cost_usd: number;
  /** Model with the most output tokens — surfaced via the row's badge. */
  dominant_model: string;
}

/**
 * Group raw assistant-message events into one row per Claude Code session.
 * Each unique `source_file` (a JSONL path under ~/.claude/projects/) is one
 * session; the lines inside it are the conversation turns. Without this,
 * "5447 sessions" actually meant 5447 individual API calls.
 */
function aggregateSessions(events: SessionEvent[]): AggregatedSession[] {
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
        total_tokens: 0,
        total_cost_usd: 0,
        dominant_model: e.model,
        _modelTokens: new Map(),
      };
      byFile.set(id, agg);
    }
    agg.turn_count += 1;
    // Total must include cache tokens — otherwise the displayed number
    // looks tiny next to the cost. Cache writes especially can dominate
    // the bill on long Claude Code sessions (50–200K tokens per turn at
    // $3.75/MTok Sonnet or $18.75/MTok Opus). Excluding them made users
    // think the cost calc was broken when actually the count was.
    agg.total_tokens +=
      e.input_tokens +
      e.output_tokens +
      e.cache_read_tokens +
      e.cache_creation_5m_tokens +
      e.cache_creation_1h_tokens;
    agg.total_cost_usd += e.cost_usd;
    if (e.ts > agg.latest_ts) agg.latest_ts = e.ts;
    agg._modelTokens.set(
      e.model,
      (agg._modelTokens.get(e.model) ?? 0) + e.output_tokens,
    );
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

export function SessionsTab() {
  const version = useAppStore((s) => s.sessionDataVersion);
  const { data: events, error, loading, reload } = useTabData(
    () => ipc.getSessionHistory(7),
    [version],
  );

  const sessions = useMemo(
    () => aggregateSessions(events ?? []),
    [events],
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
          return (
            <div
              key={session.id}
              className={[
                'flex items-center gap-[var(--space-md)]',
                'px-[var(--space-sm)] py-[var(--space-sm)]',
                'border-b border-[var(--color-border-subtle)]',
                'transition-[background] duration-[var(--duration-fast)]',
                'hover:bg-[var(--color-bg-card)]',
              ].join(' ')}
            >
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
                  {formatTokens(session.total_tokens)}
                </span>
                <span className="mono text-[length:var(--text-label)] text-[color:var(--color-text-muted)] tabular-nums min-w-[48px] text-right">
                  {formatCost(session.total_cost_usd)}
                </span>
              </div>
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

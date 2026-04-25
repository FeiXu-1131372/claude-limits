import { useEffect, useMemo, useState } from 'react';
import { Badge } from '../components/ui/Badge';
import { EmptyState } from '../components/ui/EmptyState';
import type { SessionEvent } from '../lib/types';
import { formatTokens, formatCost } from '../lib/format';
import { IconSessions } from '../lib/icons';
import { ipc } from '../lib/ipc';

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

export function SessionsTab() {
  const [events, setEvents] = useState<SessionEvent[] | null>(null);

  useEffect(() => {
    ipc.getSessionHistory(7).then(setEvents).catch(() => setEvents([]));
  }, []);

  const totalCost = useMemo(
    () => (events ?? []).reduce((sum, s) => sum + s.cost_usd, 0),
    [events],
  );

  if (events === null) {
    return <p className="text-[var(--color-text-muted)]">Loading...</p>;
  }

  if (events.length === 0) {
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
        <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">
          {events.length} sessions
        </span>
        <span className="mono text-[var(--text-label)] text-[var(--color-text-secondary)]">
          {formatCost(totalCost)}
        </span>
      </div>

      <div className="flex flex-col">
        {events.map((session, i) => {
          const total = session.input_tokens + session.output_tokens;
          const key = modelKey(session.model);
          return (
            <div
              key={`${session.source_file}-${session.source_line}-${i}`}
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
                  <span className="text-[var(--text-body)] text-[var(--color-text)] truncate">
                    {session.project}
                  </span>
                  <Badge variant={MODEL_BADGE[key] ?? 'default'}>
                    {key}
                  </Badge>
                </div>
                <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">
                  {formatTime(session.ts)}
                </span>
              </div>

              <div className="flex items-center gap-[var(--space-md)] shrink-0">
                <span className="mono text-[var(--text-label)] text-[var(--color-text-secondary)] tabular-nums">
                  {formatTokens(total)}
                </span>
                <span className="mono text-[var(--text-label)] text-[var(--color-text-muted)] tabular-nums min-w-[48px] text-right">
                  {formatCost(session.cost_usd)}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

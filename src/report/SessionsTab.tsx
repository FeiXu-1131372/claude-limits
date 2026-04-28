import { useMemo } from 'react';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { EmptyState } from '../components/ui/EmptyState';
import { formatTokens, formatCost } from '../lib/format';
import { IconSessions } from '../lib/icons';
import { ipc } from '../lib/ipc';
import { useTabData } from '../lib/useTabData';
import { useAppStore } from '../lib/store';

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
  const version = useAppStore((s) => s.sessionDataVersion);
  const { data: events, error, loading, reload } = useTabData(
    () => ipc.getSessionHistory(7),
    [version],
  );

  const totalCost = useMemo(
    () => (events ?? []).reduce((sum, s) => sum + s.cost_usd, 0),
    [events],
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
        <span className="text-[length:var(--text-label)] text-[color:var(--color-text-muted)]">
          {events.length} sessions
        </span>
        <span className="mono text-[length:var(--text-label)] text-[color:var(--color-text-secondary)]">
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
                  <span className="text-[length:var(--text-body)] text-[color:var(--color-text)] truncate">
                    {session.project}
                  </span>
                  <Badge variant={MODEL_BADGE[key] ?? 'default'}>
                    {key}
                  </Badge>
                </div>
                <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
                  {formatTime(session.ts)}
                </span>
              </div>

              <div className="flex items-center gap-[var(--space-md)] shrink-0">
                <span className="mono text-[length:var(--text-label)] text-[color:var(--color-text-secondary)] tabular-nums">
                  {formatTokens(total)}
                </span>
                <span className="mono text-[length:var(--text-label)] text-[color:var(--color-text-muted)] tabular-nums min-w-[48px] text-right">
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

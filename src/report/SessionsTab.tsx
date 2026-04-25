import { useMemo } from 'react';
import { Badge } from '../components/ui/Badge';
import { EmptyState } from '../components/ui/EmptyState';
import type { SessionEvent } from '../lib/types';
import { formatTokens, formatCost } from '../lib/format';
import { IconSessions } from '../lib/icons';

const MODEL_BADGE: Record<string, 'opus' | 'sonnet' | 'haiku'> = {
  opus: 'opus',
  sonnet: 'sonnet',
  haiku: 'haiku',
};

function formatTime(iso: string): string {
  const d = new Date(iso);
  const now = new Date();
  const isToday = d.toDateString() === now.toDateString();
  const time = d.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
  if (isToday) return time;
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }) + ' ' + time;
}

/* Placeholder data */
const PLACEHOLDER_SESSIONS: SessionEvent[] = Array.from({ length: 25 }, (_, i) => ({
  ts: new Date(Date.now() - i * 2400000).toISOString(),
  project: ['api-server', 'web-app', 'cli-tool', 'data-pipeline'][i % 4],
  model: (['opus', 'sonnet', 'haiku'] as const)[i % 3],
  input_tokens: 2000 + Math.floor(Math.random() * 8000),
  output_tokens: 1000 + Math.floor(Math.random() * 4000),
  cache_read_tokens: Math.floor(Math.random() * 3000),
  cache_creation_5m_tokens: Math.floor(Math.random() * 1000),
  cache_creation_1h_tokens: Math.floor(Math.random() * 500),
  cost_usd: 0.02 + Math.random() * 0.15,
  source_file: `~/.claude/projects/${['api-server', 'web-app', 'cli-tool', 'data-pipeline'][i % 4]}/session.jsonl`,
  source_line: i * 256,
}));

export function SessionsTab() {
  const sessions = PLACEHOLDER_SESSIONS;

  const totalCost = useMemo(
    () => sessions.reduce((sum, s) => sum + s.cost_usd, 0),
    [sessions],
  );

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
        <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">
          {sessions.length} sessions
        </span>
        <span className="mono text-[var(--text-label)] text-[var(--color-text-secondary)]">
          {formatCost(totalCost)}
        </span>
      </div>

      <div className="flex flex-col">
        {sessions.map((session, i) => {
          const total = session.input_tokens + session.output_tokens;
          return (
            <div
              key={i}
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
                  <Badge variant={MODEL_BADGE[session.model] ?? 'default'}>
                    {session.model}
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

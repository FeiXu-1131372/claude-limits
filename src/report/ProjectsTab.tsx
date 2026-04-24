import { Card } from '../components/ui/Card';
import { Badge } from '../components/ui/Badge';
import { EmptyState } from '../components/ui/EmptyState';
import type { ProjectStats } from '../lib/types';
import { formatTokens, formatCost } from '../lib/format';
import { IconChart } from '../lib/icons';

const PLACEHOLDER: ProjectStats[] = [
  { project: 'api-server', sessions: 42, total_tokens: 8_400_000, cost_usd: 12.6, models: { opus: 30, sonnet: 12 } },
  { project: 'web-app', sessions: 38, total_tokens: 5_200_000, cost_usd: 5.2, models: { sonnet: 28, haiku: 10 } },
  { project: 'cli-tool', sessions: 18, total_tokens: 2_100_000, cost_usd: 3.15, models: { opus: 12, sonnet: 6 } },
  { project: 'data-pipeline', sessions: 12, total_tokens: 1_800_000, cost_usd: 1.8, models: { haiku: 8, sonnet: 4 } },
  { project: 'scripts', sessions: 5, total_tokens: 400_000, cost_usd: 0.2, models: { haiku: 5 } },
];

export function ProjectsTab() {
  const data = PLACEHOLDER;
  const maxTokens = Math.max(...data.map((p) => p.total_tokens), 1);

  if (data.length === 0) {
    return (
      <EmptyState
        icon={<IconChart size={32} />}
        title="No project data"
        description="Projects will appear as you use Claude Code in different directories."
      />
    );
  }

  return (
    <div className="flex flex-col gap-[var(--space-sm)]">
      {data.map((project) => {
        const widthPct = (project.total_tokens / maxTokens) * 100;
        const modelKeys = Object.keys(project.models) as Array<keyof typeof project.models>;

        return (
          <Card key={project.project} className="p-[var(--space-md)]">
            <div className="flex flex-col gap-[var(--space-sm)]">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-[var(--space-sm)]">
                  <span className="text-[var(--text-body)] font-[var(--weight-medium)] text-[var(--color-text)]">
                    {project.project}
                  </span>
                  <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">
                    {project.sessions} sessions
                  </span>
                </div>
                <span className="mono text-[var(--text-label)] font-[var(--weight-semibold)] text-[var(--color-text)] tabular-nums">
                  {formatCost(project.cost_usd)}
                </span>
              </div>

              {/* Stacked bar */}
              <div className="flex h-[6px] rounded-[var(--radius-pill)] bg-[var(--color-track)] overflow-hidden gap-[1px]">
                {modelKeys.map((model) => {
                  const modelPct = (project.models[model] ?? 0) / project.sessions;
                  const colors: Record<string, string> = {
                    opus: 'var(--color-accent)',
                    sonnet: 'var(--color-warn)',
                    haiku: 'var(--color-safe)',
                  };
                  return (
                    <div
                      key={model}
                      className="h-full rounded-[var(--radius-pill)] transition-[width] duration-[var(--duration-bar)]"
                      style={{
                        width: `${modelPct * widthPct}%`,
                        background: colors[model] ?? 'var(--color-text-muted)',
                      }}
                    />
                  );
                })}
              </div>

              {/* Model badges */}
              <div className="flex gap-[var(--space-xs)]">
                {modelKeys.map((model) => (
                  <Badge key={model} variant={model === 'opus' ? 'opus' : model === 'sonnet' ? 'sonnet' : 'haiku'}>
                    {model} {project.models[model]}
                  </Badge>
                ))}
                <span className="mono text-[var(--text-micro)] text-[var(--color-text-muted)] ml-auto tabular-nums">
                  {formatTokens(project.total_tokens)}
                </span>
              </div>
            </div>
          </Card>
        );
      })}
    </div>
  );
}

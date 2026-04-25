import { Card } from '../components/ui/Card';
import { EmptyState } from '../components/ui/EmptyState';
import type { ProjectStats } from '../lib/types';
import { formatCost } from '../lib/format';
import { IconChart } from '../lib/icons';

const PLACEHOLDER: ProjectStats[] = [
  { project: 'api-server', session_count: 42, total_cost_usd: 12.6 },
  { project: 'web-app', session_count: 38, total_cost_usd: 5.2 },
  { project: 'cli-tool', session_count: 18, total_cost_usd: 3.15 },
  { project: 'data-pipeline', session_count: 12, total_cost_usd: 1.8 },
  { project: 'scripts', session_count: 5, total_cost_usd: 0.2 },
];

export function ProjectsTab() {
  const data = PLACEHOLDER;
  const maxCost = Math.max(...data.map((p) => p.total_cost_usd), 1);

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
        const widthPct = (project.total_cost_usd / maxCost) * 100;

        return (
          <Card key={project.project} className="p-[var(--space-md)]">
            <div className="flex flex-col gap-[var(--space-sm)]">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-[var(--space-sm)]">
                  <span className="text-[var(--text-body)] font-[var(--weight-medium)] text-[var(--color-text)]">
                    {project.project}
                  </span>
                  <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">
                    {project.session_count} sessions
                  </span>
                </div>
                <span className="mono text-[var(--text-label)] font-[var(--weight-semibold)] text-[var(--color-text)] tabular-nums">
                  {formatCost(project.total_cost_usd)}
                </span>
              </div>

              {/* Bar */}
              <div className="flex h-[6px] rounded-[var(--radius-pill)] bg-[var(--color-track)] overflow-hidden">
                <div
                  className="h-full rounded-[var(--radius-pill)] bg-[var(--color-accent)] transition-[width] duration-[var(--duration-bar)]"
                  style={{ width: `${widthPct}%` }}
                />
              </div>
            </div>
          </Card>
        );
      })}
    </div>
  );
}

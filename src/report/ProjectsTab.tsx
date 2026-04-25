import { Card } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { EmptyState } from '../components/ui/EmptyState';
import { formatCost } from '../lib/format';
import { IconChart } from '../lib/icons';
import { ipc } from '../lib/ipc';
import { useTabData } from '../lib/useTabData';

export function ProjectsTab() {
  const { data, error, loading, reload } = useTabData(() => ipc.getProjectBreakdown(30));

  if (error) {
    return (
      <EmptyState
        icon={<IconChart size={32} />}
        title="Couldn't load projects"
        description={error}
        action={<Button variant="ghost" size="sm" onClick={reload}>Retry</Button>}
      />
    );
  }
  if (loading || !data) {
    return <p className="text-[var(--color-text-muted)]">Loading…</p>;
  }

  if (data.length === 0) {
    return (
      <EmptyState
        icon={<IconChart size={32} />}
        title="No project data"
        description="Projects will appear as you use Claude Code in different directories."
      />
    );
  }

  const maxCost = Math.max(...data.map((p) => p.total_cost_usd), 1);

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

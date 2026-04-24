import { useState } from 'react';
import { motion } from 'framer-motion';
import { Tabs } from '../components/ui/Tabs';
import { Badge } from '../components/ui/Badge';
import { IconButton } from '../components/ui/IconButton';
import { SessionsTab } from './SessionsTab';
import { ModelsTab } from './ModelsTab';
import { TrendsTab } from './TrendsTab';
import { ProjectsTab } from './ProjectsTab';
import { HeatmapTab } from './HeatmapTab';
import { CacheTab } from './CacheTab';
import { useStore } from '../lib/store';
import { tabSlide } from '../lib/motion';
import {
  IconSessions,
  IconChart,
  IconTrends,
  IconPolling,
  IconHeatmap,
  IconCache,
  IconRefresh,
} from '../lib/icons';

const TAB_CONFIG = [
  { id: 'sessions', label: 'Sessions', icon: IconSessions },
  { id: 'models', label: 'Models', icon: IconChart },
  { id: 'trends', label: 'Trends', icon: IconTrends },
  { id: 'projects', label: 'Projects', icon: IconPolling },
  { id: 'heatmap', label: 'Heatmap', icon: IconHeatmap },
  { id: 'cache', label: 'Cache', icon: IconCache },
] as const;

const TAB_COMPONENTS: Record<string, React.FC> = {
  sessions: SessionsTab,
  models: ModelsTab,
  trends: TrendsTab,
  projects: ProjectsTab,
  heatmap: HeatmapTab,
  cache: CacheTab,
};

export function ExpandedReport() {
  const [activeTab, setActiveTab] = useState('sessions');
  const snapshot = useStore((s) => s.snapshot);

  const TabComponent = TAB_COMPONENTS[activeTab] ?? SessionsTab;

  return (
    <div
      className="flex flex-col h-full bg-[var(--color-bg-surface)] rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] overflow-hidden"
      style={{
        width: 'var(--report-width)',
        minHeight: 'var(--report-min-height)',
      }}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-[var(--space-lg)] py-[var(--space-md)] border-b border-[var(--color-border-subtle)] shrink-0">
        <div className="flex items-center gap-[var(--space-sm)]">
          <span className="text-[var(--text-body)] font-[var(--weight-semibold)] text-[var(--color-text)]">
            Claude Usage Report
          </span>
          <Badge variant={snapshot?.is_stale ? 'stale' : 'live'}>
            {snapshot?.is_stale ? 'Stale' : 'Live'}
          </Badge>
        </div>
        <div className="flex items-center gap-[var(--space-sm)]">
          <span className="mono text-[var(--text-label)] text-[var(--color-text-muted)]">
            Last 30 days
          </span>
          <IconButton label="Refresh">
            <IconRefresh size={14} />
          </IconButton>
        </div>
      </div>

      {/* Tab bar */}
      <div className="px-[var(--space-lg)] pt-[var(--space-md)] shrink-0">
        <div className="flex gap-[var(--space-2xs)] p-[var(--space-2xs)] bg-[var(--color-track)] rounded-[var(--radius-md)]">
          {TAB_CONFIG.map((tab) => {
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={[
                  'flex items-center gap-[var(--space-2xs)]',
                  'px-[var(--space-sm)] py-[var(--space-xs)]',
                  'text-[var(--text-label)] font-[var(--weight-medium)]',
                  'rounded-[var(--radius-sm)] select-none whitespace-nowrap',
                  'transition-[background,color] duration-[var(--duration-fast)] ease-[var(--ease-out)]',
                  isActive
                    ? 'bg-[var(--color-bg-card)] text-[var(--color-text)]'
                    : 'text-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)]',
                  'focus-visible:outline-2 focus-visible:outline-[var(--color-border-focus)] focus-visible:outline-offset-1',
                ].join(' ')}
              >
                <tab.icon size={12} />
                {tab.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* Tab content */}
      <div className="flex-1 overflow-y-auto px-[var(--space-lg)] pb-[var(--space-lg)]">
        <motion.div
          key={activeTab}
          variants={tabSlide}
          initial="enter"
          animate="center"
          exit="exit"
          custom={1}
          className="pt-[var(--space-md)]"
        >
          <TabComponent />
        </motion.div>
      </div>
    </div>
  );
}

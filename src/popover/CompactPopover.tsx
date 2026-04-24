import { useMemo } from 'react';
import { motion } from 'framer-motion';
import { Card } from '../components/ui/Card';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { IconButton } from '../components/ui/IconButton';
import { UsageBar } from './UsageBar';
import { useStore } from '../lib/store';
import { popoverMount, cardStagger, cardChild } from '../lib/motion';
import { IconRefresh, IconSettings } from '../lib/icons';

function formatRelativeTime(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  return `${hours}h ago`;
}

function formatResetCountdown(iso: string): string {
  const diff = new Date(iso).getTime() - Date.now();
  if (diff <= 0) return 'resetting...';
  const hours = Math.floor(diff / 3600000);
  const mins = Math.floor((diff % 3600000) / 60000);
  const days = Math.floor(hours / 24);
  if (days > 0) return `${days}d ${hours % 24}h left`;
  if (hours > 0) return `${hours}h ${mins}m left`;
  return `${mins}m left`;
}

const MODELS = [
  { key: 'opus' as const, label: 'Opus', color: 'var(--color-accent)' },
  { key: 'sonnet' as const, label: 'Sonnet', color: 'var(--color-warn)' },
  { key: 'haiku' as const, label: 'Haiku', color: 'var(--color-safe)' },
];

export function CompactPopover() {
  const snapshot = useStore((s) => s.snapshot);
  const setShowSettings = useStore((s) => s.setShowSettings);

  const fiveTimer = useMemo(
    () => (snapshot?.five_hour.reset_at ? formatResetCountdown(snapshot.five_hour.reset_at) : ''),
    [snapshot?.five_hour.reset_at],
  );

  const sevenTimer = useMemo(
    () => (snapshot?.seven_day.reset_at ? formatResetCountdown(snapshot.seven_day.reset_at) : ''),
    [snapshot?.seven_day.reset_at],
  );

  const updatedAgo = useMemo(
    () => (snapshot?.fetched_at ? formatRelativeTime(snapshot.fetched_at) : ''),
    [snapshot?.fetched_at],
  );

  if (!snapshot) return null;

  return (
    <motion.div
      className="relative flex flex-col gap-[var(--space-sm)] p-[var(--space-lg)]"
      style={{
        width: 'var(--popover-width)',
        height: 'var(--popover-height)',
      }}
      variants={popoverMount}
      initial="hidden"
      animate="visible"
      exit="exit"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-[var(--space-2xs)]">
        <div className="flex items-center gap-[var(--space-sm)]">
          <span className="text-[var(--text-body)] font-[var(--weight-semibold)] text-[var(--color-text)]">
            Claude
          </span>
          <Badge variant={snapshot.is_stale ? 'stale' : 'live'}>
            {snapshot.is_stale ? 'Stale' : 'Live'}
          </Badge>
        </div>
        <div className="flex items-center gap-[var(--space-2xs)]">
          <IconButton label="Refresh">
            <IconRefresh size={14} />
          </IconButton>
          <IconButton label="Settings" onClick={() => setShowSettings(true)}>
            <IconSettings size={14} />
          </IconButton>
        </div>
      </div>

      <motion.div
        className="flex flex-col gap-[var(--space-sm)] flex-1"
        variants={cardStagger}
        initial="hidden"
        animate="visible"
      >
        {/* Usage card */}
        <motion.div variants={cardChild}>
          <Card className="p-[var(--space-md)]">
            <div className="flex flex-col gap-[var(--space-md)]">
              <UsageBar
                label="5h window"
                value={snapshot.five_hour.used_pct}
                timer={fiveTimer}
              />
              <UsageBar
                label="7d window"
                value={snapshot.seven_day.used_pct}
                timer={sevenTimer}
              />
            </div>
          </Card>
        </motion.div>

        {/* Models row — dot + label + number, no double nesting */}
        <motion.div variants={cardChild}>
          <Card className="p-[var(--space-sm)]">
            <div className="flex gap-[var(--space-md)] px-[var(--space-2xs)]">
              {MODELS.map(({ key, label, color }) => {
                const pct = snapshot.per_model[key]?.used_pct ?? 0;
                return (
                  <div key={key} className="flex items-center gap-[var(--space-xs)] flex-1">
                    <span
                      className="w-[6px] h-[6px] rounded-full shrink-0"
                      style={{ background: color }}
                    />
                    <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">
                      {label}
                    </span>
                    <span className="mono text-[var(--text-label)] font-[var(--weight-semibold)] text-[var(--color-text)] tabular-nums ml-auto">
                      {Math.round(pct)}%
                    </span>
                  </div>
                );
              })}
            </div>
          </Card>
        </motion.div>
      </motion.div>

      {/* Footer */}
      <div className="flex items-center justify-between px-[var(--space-2xs)] pt-[var(--space-xs)]">
        <span className="mono text-[var(--text-micro)] text-[var(--color-text-muted)]">
          Updated {updatedAgo}
        </span>
        <Button variant="primary" size="sm">
          See details
        </Button>
      </div>
    </motion.div>
  );
}

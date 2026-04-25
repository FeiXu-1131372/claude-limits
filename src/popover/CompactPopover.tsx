import { useMemo } from 'react';
import { motion } from 'framer-motion';
import { Card } from '../components/ui/Card';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { Banner } from '../components/ui/Banner';
import { IconButton } from '../components/ui/IconButton';
import { ProgressBar } from '../components/ui/ProgressBar';
import { UsageBar } from './UsageBar';
import { useAppStore } from '../lib/store';
import { ipc } from '../lib/ipc';
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

export function CompactPopover() {
  const usage = useAppStore((s) => s.usage);
  const thresholds = useAppStore((s) => s.settings?.thresholds ?? [75, 90]);
  const authRequired = useAppStore((s) => s.authRequired);
  const stale = useAppStore((s) => s.stale);
  const conflict = useAppStore((s) => s.conflict);
  const dismissBanner = useAppStore((s) => s.dismissBanner);

  if (!usage) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <span className="text-[var(--color-text-muted)]">Loading usage...</span>
      </div>
    );
  }

  const snap = usage.snapshot;
  const extra = snap.extra_usage;

  const updatedAgo = useMemo(
    () => (snap.fetched_at ? formatRelativeTime(snap.fetched_at) : ''),
    [snap.fetched_at],
  );

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
      {/* Banners */}
      {authRequired && (
        <Banner variant="warning" onDismiss={() => dismissBanner('authRequired')}>
          Sign in to continue monitoring.
        </Banner>
      )}
      {stale && (
        <Banner variant="stale" onDismiss={() => dismissBanner('stale')}>
          Data may be stale.
        </Banner>
      )}
      {conflict && (
        <Banner variant="warning">
          Two accounts detected — choose which to monitor in Settings.
        </Banner>
      )}

      {/* Header */}
      <div className="flex items-center justify-between px-[var(--space-2xs)]">
        <div className="flex items-center gap-[var(--space-sm)]">
          <span className="text-[var(--text-body)] font-[var(--weight-semibold)] text-[var(--color-text)]">
            Claude
          </span>
          <Badge variant={stale ? 'stale' : 'live'}>
            {stale ? 'Stale' : 'Live'}
          </Badge>
        </div>
        <div className="flex items-center gap-[var(--space-2xs)]">
          <IconButton label="Refresh">
            <IconRefresh size={14} />
          </IconButton>
          <IconButton label="Settings">
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
        {/* Primary usage card */}
        <motion.div variants={cardChild}>
          <Card className="p-[var(--space-md)]">
            <div className="flex flex-col gap-[var(--space-md)]">
              <UsageBar
                label="5h window"
                data={snap.five_hour}
                warnAt={thresholds[0] ?? 75}
                dangerAt={thresholds[1] ?? 90}
              />
              <UsageBar
                label="7d window"
                data={snap.seven_day}
                warnAt={thresholds[0] ?? 75}
                dangerAt={thresholds[1] ?? 90}
              />
              {(snap.seven_day_opus || snap.seven_day_sonnet) && (
                <div className="grid grid-cols-2 gap-[var(--space-md)] border-t border-[var(--color-border-subtle)] pt-[var(--space-sm)]">
                  <UsageBar
                    label="Opus"
                    data={snap.seven_day_opus}
                    warnAt={thresholds[0] ?? 75}
                    dangerAt={thresholds[1] ?? 90}
                    size="sm"
                  />
                  <UsageBar
                    label="Sonnet"
                    data={snap.seven_day_sonnet}
                    warnAt={thresholds[0] ?? 75}
                    dangerAt={thresholds[1] ?? 90}
                    size="sm"
                  />
                </div>
              )}
              {extra?.is_enabled && (
                <ExtraUsageSection
                  pct={extra.utilization}
                  resetsAt={extra.resets_at ?? null}
                  thresholds={thresholds}
                />
              )}
            </div>
          </Card>
        </motion.div>
      </motion.div>

      {/* Footer */}
      <div className="flex items-center justify-between px-[var(--space-2xs)] pt-[var(--space-xs)]">
        <span className="mono text-[var(--text-micro)] text-[var(--color-text-muted)]">
          Updated {updatedAgo}
        </span>
        <Button variant="primary" size="sm" onClick={() => ipc.openExpandedWindow()}>
          See details
        </Button>
      </div>
    </motion.div>
  );
}

function ExtraUsageSection({
  pct,
  resetsAt,
  thresholds,
}: {
  pct: number;
  resetsAt: string | null;
  thresholds: number[];
}) {
  if (resetsAt) {
    return (
      <UsageBar
        label="Pay-as-you-go"
        data={{ utilization: pct, resets_at: resetsAt }}
        warnAt={thresholds[0] ?? 75}
        dangerAt={thresholds[1] ?? 90}
        size="sm"
      />
    );
  }
  const rounded = Math.round(pct);
  return (
    <div className="border-t border-[var(--color-border-subtle)] pt-[var(--space-sm)]">
      <div className="flex items-center justify-between mb-1">
        <span className="text-[var(--text-label)] text-[var(--color-text-secondary)]">Pay-as-you-go</span>
        <span className="mono text-[var(--text-label)] tabular-nums text-[var(--color-text)]">
          {rounded}%
        </span>
      </div>
      <ProgressBar
        value={rounded}
        warnThreshold={thresholds[0] ?? 75}
        dangerThreshold={thresholds[1] ?? 90}
        showLabel={false}
      />
      <div className="flex justify-end mt-1">
        <span className="mono text-[var(--text-micro)] text-[var(--color-text-muted)]">No reset window</span>
      </div>
    </div>
  );
}

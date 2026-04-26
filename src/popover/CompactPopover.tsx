import { useMemo, useState } from 'react';
import { motion } from 'framer-motion';
import { Banner } from '../components/ui/Banner';
import { IconButton } from '../components/ui/IconButton';
import { SettingsPanel } from '../settings/SettingsPanel';
import { useAppStore } from '../lib/store';
import { ipc } from '../lib/ipc';
import { IconRefresh, IconSettings, ChevronRight } from '../lib/icons';
import { InstrumentColumn, InstrumentRow } from './InstrumentRow';
import type { Utilization } from '../lib/types';

function formatRelativeTime(iso: string): string {
  const t = new Date(iso).getTime();
  if (!Number.isFinite(t)) return '';
  const diff = Date.now() - t;
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
  const [view, setView] = useState<'home' | 'settings'>('home');
  const [refreshing, setRefreshing] = useState(false);

  const fetchedAt = usage?.snapshot.fetched_at;
  const updatedAgo = useMemo(
    () => (fetchedAt ? formatRelativeTime(fetchedAt) : ''),
    [fetchedAt],
  );

  async function handleRefresh() {
    if (refreshing) return;
    setRefreshing(true);
    try {
      await ipc.forceRefresh();
    } finally {
      setTimeout(() => setRefreshing(false), 420);
    }
  }

  if (view === 'settings') {
    return (
      <Shell>
        <Header title="Settings" onBack={() => setView('home')} />
        <div className="flex-1 overflow-y-auto px-[var(--popover-pad)] pb-[var(--space-md)]">
          <SettingsPanel />
        </div>
      </Shell>
    );
  }

  if (!usage) {
    return (
      <Shell>
        <ChromeBar
          live={false}
          stale={false}
          refreshing={refreshing}
          onRefresh={handleRefresh}
          onSettings={() => setView('settings')}
        />
        <div className="flex flex-1 items-center justify-center">
          <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">
            Loading…
          </span>
        </div>
      </Shell>
    );
  }

  const snap = usage.snapshot;
  const extra = snap.extra_usage;
  const warn = thresholds[0] ?? 75;
  const danger = thresholds[1] ?? 90;

  return (
    <Shell>
      <ChromeBar
        live
        stale={stale}
        refreshing={refreshing}
        onRefresh={handleRefresh}
        onSettings={() => setView('settings')}
      />

      {/* Banners — quiet, hairline border, dismissible */}
      <div className="flex flex-col gap-[6px] px-[var(--popover-pad)]">
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
            Two accounts detected — choose in Settings.
          </Banner>
        )}
      </div>

      {/* Hero: two-column instrument readout */}
      <motion.div
        initial={{ opacity: 0, y: 6 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.32, ease: [0.22, 1, 0.36, 1] }}
        className="grid grid-cols-2 gap-x-[var(--space-lg)] px-[var(--popover-pad)] pt-[var(--space-md)] pb-[var(--space-lg)]"
      >
        <InstrumentColumn
          label="5h"
          data={snap.five_hour}
          warnAt={warn}
          dangerAt={danger}
        />
        <InstrumentColumn
          label="7d"
          data={snap.seven_day}
          warnAt={warn}
          dangerAt={danger}
        />
      </motion.div>

      {/* Opus / Sonnet sub-row — only if 7d split data exists */}
      {(snap.seven_day_opus || snap.seven_day_sonnet) && (
        <>
          <Hairline />
          <div className="grid grid-cols-2 gap-x-[var(--space-lg)] px-[var(--popover-pad)] py-[var(--space-sm)]">
            <InstrumentRow label="Opus" data={snap.seven_day_opus} warnAt={warn} dangerAt={danger} />
            <InstrumentRow label="Sonnet" data={snap.seven_day_sonnet} warnAt={warn} dangerAt={danger} />
          </div>
        </>
      )}

      {/* Pay-as-you-go — its own row, hairline-divided */}
      {extra?.is_enabled && (
        <>
          <Hairline />
          <div className="px-[var(--popover-pad)] py-[var(--space-sm)]">
            <ExtraRow
              pct={extra.utilization ?? 0}
              resetsAt={extra.resets_at ?? null}
              warnAt={warn}
              dangerAt={danger}
            />
          </div>
        </>
      )}

      {/* Footer: timestamp + ghost link to expanded report */}
      <div className="mt-auto flex items-center justify-between px-[var(--popover-pad)] py-[var(--space-sm)] border-t border-[var(--color-rule)]">
        <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">
          Updated {updatedAgo || '—'}
        </span>
        <button
          type="button"
          onClick={() => ipc.openExpandedWindow()}
          className="
            group inline-flex items-center gap-[2px]
            text-[var(--text-label)] text-[var(--color-text-secondary)]
            transition-colors duration-[var(--duration-fast)] ease-[var(--ease-out)]
            hover:text-[var(--color-text)]
            focus-visible:outline-2 focus-visible:outline-[var(--color-border-focus)] focus-visible:outline-offset-2 rounded
          "
        >
          See details
          <ChevronRight
            size={11}
            className="transition-transform duration-[var(--duration-fast)] ease-[var(--ease-out)] group-hover:translate-x-[1px]"
          />
        </button>
      </div>
    </Shell>
  );
}

/* ───────────────────────── Sub-components ───────────────────────── */

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="relative flex h-full w-full flex-col"
      style={{ width: 'var(--popover-width)', height: 'var(--popover-height)' }}
    >
      {children}
    </div>
  );
}

/**
 * Top chrome strip — also serves as the drag region for the borderless
 * popover window. The whole strip is `data-tauri-drag-region`; buttons inside
 * intercept their own clicks normally.
 */
function ChromeBar({
  live,
  stale,
  refreshing,
  onRefresh,
  onSettings,
}: {
  live: boolean;
  stale: boolean;
  refreshing: boolean;
  onRefresh: () => void;
  onSettings: () => void;
}) {
  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between gap-[var(--space-sm)] px-[var(--popover-pad)] pt-[var(--space-md)] pb-[var(--space-sm)] cursor-default select-none"
    >
      <div data-tauri-drag-region className="flex items-center gap-[var(--space-xs)]">
        <span className="text-[var(--text-label)] font-[var(--weight-semibold)] text-[var(--color-text-secondary)] tracking-[var(--tracking-label)] uppercase">
          Claude
        </span>
        <StatusDot live={live} stale={stale} />
      </div>
      <div className="flex items-center gap-[2px]">
        <IconButton label="Refresh" onClick={onRefresh}>
          <motion.span
            animate={refreshing ? { rotate: 360 } : { rotate: 0 }}
            transition={
              refreshing
                ? { duration: 0.7, ease: 'linear', repeat: Infinity }
                : { duration: 0.2 }
            }
            style={{ display: 'inline-flex' }}
          >
            <IconRefresh size={13} />
          </motion.span>
        </IconButton>
        <IconButton label="Settings" onClick={onSettings}>
          <IconSettings size={13} />
        </IconButton>
      </div>
    </div>
  );
}

function Header({ title, onBack }: { title: string; onBack: () => void }) {
  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between gap-[var(--space-sm)] px-[var(--popover-pad)] pt-[var(--space-md)] pb-[var(--space-sm)] cursor-default select-none"
    >
      <button
        type="button"
        onClick={onBack}
        className="
          inline-flex items-center gap-[var(--space-2xs)]
          text-[var(--text-label)] text-[var(--color-text-secondary)] tracking-[var(--tracking-label)] uppercase
          transition-colors duration-[var(--duration-fast)]
          hover:text-[var(--color-text)]
          focus-visible:outline-2 focus-visible:outline-[var(--color-border-focus)] focus-visible:outline-offset-2 rounded
        "
      >
        <ChevronRight size={11} className="rotate-180" />
        Back
      </button>
      <span className="text-[var(--text-label)] font-[var(--weight-semibold)] text-[var(--color-text-secondary)] tracking-[var(--tracking-label)] uppercase">
        {title}
      </span>
      <span className="w-[40px]" /> {/* visual ballast for centering */}
    </div>
  );
}

/**
 * Live = teal pulse. Stale = warm dimmed dot, no pulse. Offline = transparent
 * ring. Replaces the previous pill badge — quieter, doesn't compete with data.
 */
function StatusDot({ live, stale }: { live: boolean; stale: boolean }) {
  if (stale) {
    return (
      <span
        title="Stale"
        className="inline-block h-[6px] w-[6px] rounded-full"
        style={{ background: 'var(--color-warn)' }}
      />
    );
  }
  if (!live) {
    return (
      <span
        title="Offline"
        className="inline-block h-[6px] w-[6px] rounded-full ring-1 ring-[var(--color-rule-strong)]"
      />
    );
  }
  return (
    <span className="relative inline-flex h-[6px] w-[6px] items-center justify-center" title="Live">
      <span
        className="absolute inline-block h-[6px] w-[6px] rounded-full opacity-60"
        style={{ background: 'var(--color-accent)', animation: 'pulse-dot 2.4s ease-in-out infinite' }}
      />
      <span
        className="relative inline-block h-[6px] w-[6px] rounded-full"
        style={{ background: 'var(--color-accent)' }}
      />
    </span>
  );
}

function Hairline() {
  return <div className="mx-[var(--popover-pad)] border-t border-[var(--color-rule)]" />;
}

/* ───────────────────────── Pay-as-you-go row ───────────────────────── */

function ExtraRow({
  pct,
  resetsAt,
  warnAt,
  dangerAt,
}: {
  pct: number;
  resetsAt: string | null;
  warnAt: number;
  dangerAt: number;
}) {
  const data: Utilization | null = resetsAt
    ? { utilization: pct, resets_at: resetsAt }
    : null;
  return (
    <InstrumentRow
      label="Pay-as-you-go"
      caption={resetsAt ? undefined : 'no reset window'}
      value={pct}
      data={data}
      warnAt={warnAt}
      dangerAt={dangerAt}
    />
  );
}

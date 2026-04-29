import { useEffect, useMemo, useState } from 'react';
import { motion } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { Banner } from '../components/ui/Banner';
import { IconButton } from '../components/ui/IconButton';
import { UpdateBanner } from '../components/UpdateBanner';
import { UsageSummary } from '../components/UsageSummary';
import { SettingsPanel } from '../settings/SettingsPanel';
import { useAppStore } from '../lib/store';
import { useUpdateStore } from '../state/updateStore';
import { ipc } from '../lib/ipc';
import { IconRefresh, IconSettings, ChevronRight, X, IconExpand } from '../lib/icons';
import { handleDragStart, closeWindow } from '../lib/window-chrome';

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
  const toggleViewMode = useAppStore((s) => s.toggleViewMode);
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
        <div className="flex-1 overflow-y-auto px-[var(--popover-pad)] pb-[var(--space-md)] pt-[var(--space-xs)]">
          <SettingsPanel />
        </div>
      </Shell>
    );
  }

  if (!usage) {
    return <LoadingShell refreshing={refreshing} onRefresh={handleRefresh} onSettings={() => setView('settings')} />;
  }

  const warn = thresholds[0] ?? 75;
  const danger = thresholds[1] ?? 90;

  return (
    <Shell>
      <UpdateBanner />
      <ChromeBar
        live
        stale={stale}
        refreshing={refreshing}
        onRefresh={handleRefresh}
        onSettings={() => setView('settings')}
        onToggleView={toggleViewMode}
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

      <UsageSummary usage={usage} thresholds={[warn, danger]} />

      {/* Footer: timestamp left, version + check-for-updates right. Explicit
       * inline marginTop: auto because Tailwind's `mt-auto` utility wasn't
       * being picked up here — this is more robust than depending on JIT. */}
      <div
        style={{ marginTop: 'auto' }}
        className="flex items-center justify-between gap-2 px-[var(--popover-pad)] py-[var(--space-sm)] border-t border-[var(--color-rule)]"
      >
        <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
          Updated {updatedAgo || '—'}
        </span>
        <VersionFooter />
      </div>
    </Shell>
  );
}


/**
 * Shown while `usage` is still null. Self-heals by:
 *   1. Kicking ipc.forceRefresh() the moment it mounts so the Rust poll loop
 *      wakes up and emits usage_updated immediately, instead of waiting for
 *      the next scheduled poll (default 5 min).
 *   2. Polling ipc.getCurrentUsage() every second in case a poll already
 *      succeeded but its event landed before the webview was listening
 *      (Tauri can drop events fired before the listener registers).
 *   3. Showing the user a clear "Tap refresh if this hangs" affordance after
 *      ~3s so they're not staring at a spinner indefinitely.
 */
function LoadingShell({
  refreshing,
  onRefresh,
  onSettings,
}: {
  refreshing: boolean;
  onRefresh: () => void;
  onSettings: () => void;
}) {
  const refreshUsage = useAppStore((s) => s.refreshUsage);
  const [hint, setHint] = useState(false);

  useEffect(() => {
    // Wake the poll loop now.
    ipc.forceRefresh().catch(() => {});

    // Fall back to polling the cached snapshot in case the usage_updated
    // event was emitted before this webview's listener registered.
    const tick = setInterval(() => {
      refreshUsage().catch(() => {});
    }, 1000);

    // Surface a manual hint if loading drags.
    const hintTimer = setTimeout(() => setHint(true), 3000);

    return () => {
      clearInterval(tick);
      clearTimeout(hintTimer);
    };
  }, [refreshUsage]);

  return (
    <Shell>
      <ChromeBar
        live={false}
        stale={false}
        refreshing={refreshing}
        onRefresh={onRefresh}
        onSettings={onSettings}
      />
      <div className="flex flex-1 flex-col items-center justify-center gap-[var(--space-sm)] px-[var(--popover-pad)] text-center">
        <span className="text-[length:var(--text-label)] text-[color:var(--color-text-muted)]">
          Loading usage…
        </span>
        {hint && (
          <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)] opacity-70">
            Taking longer than expected — tap the refresh icon.
          </span>
        )}
      </div>
    </Shell>
  );
}

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
  onToggleView,
}: {
  live: boolean;
  stale: boolean;
  refreshing: boolean;
  onRefresh: () => void;
  onSettings: () => void;
  onToggleView?: () => void;
}) {
  return (
    <div
      onPointerDown={handleDragStart}
      className="flex items-center justify-between gap-[var(--space-sm)] px-[var(--popover-pad)] pt-[var(--space-md)] pb-[var(--space-sm)] cursor-default select-none"
    >
      <div className="flex items-center gap-[var(--space-xs)] pointer-events-none">
        <span className="text-[length:var(--text-label)] font-[var(--weight-semibold)] text-[color:var(--color-text-secondary)] tracking-[var(--tracking-label)] uppercase">
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
        {onToggleView && (
          <IconButton label="Expand details" onClick={onToggleView}>
            <IconExpand size={13} />
          </IconButton>
        )}
        <IconButton label="Close" onClick={closeWindow}>
          <X size={13} />
        </IconButton>
      </div>
    </div>
  );
}

function Header({ title, onBack }: { title: string; onBack: () => void }) {
  return (
    <div
      onPointerDown={handleDragStart}
      className="flex items-center justify-between gap-[var(--space-sm)] px-[var(--popover-pad)] pt-[var(--space-md)] pb-[var(--space-sm)] cursor-default select-none"
    >
      <button
        type="button"
        onClick={onBack}
        className="
          inline-flex items-center gap-[var(--space-2xs)]
          text-[length:var(--text-label)] text-[color:var(--color-text-secondary)] tracking-[var(--tracking-label)] uppercase
          transition-colors duration-[var(--duration-fast)]
          hover:text-[color:var(--color-text)]
          focus-visible:outline-2 focus-visible:outline-[var(--color-border-focus)] focus-visible:outline-offset-2 rounded
        "
      >
        <ChevronRight size={11} className="rotate-180" />
        Back
      </button>
      <span className="text-[length:var(--text-label)] font-[var(--weight-semibold)] text-[color:var(--color-text-secondary)] tracking-[var(--tracking-label)] uppercase">
        {title}
      </span>
      <IconButton label="Close" onClick={closeWindow}>
        <X size={13} />
      </IconButton>
    </div>
  );
}

/**
 * Live = accent pulse. Stale = warm dimmed dot, no pulse. Offline = transparent
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

/**
 * Right side of the footer — shows current version and a transient
 * "Check for updates" link that reflects the live update status.
 */
function VersionFooter() {
  const status = useUpdateStore((s) => s.status);
  const [transient, setTransient] = useState<null | 'checking' | 'up-to-date' | 'failed'>(null);

  useEffect(() => {
    if (status === 'checking') {
      setTransient('checking');
      return;
    }
    if (status === 'up-to-date') {
      setTransient('up-to-date');
      const t = setTimeout(() => setTransient(null), 3000);
      return () => clearTimeout(t);
    }
    if (status === 'failed') {
      setTransient('failed');
      const t = setTimeout(() => setTransient(null), 3000);
      return () => clearTimeout(t);
    }
    if (status === 'available' || status === 'downloading') {
      setTransient('checking');
      return;
    }
    setTransient(null);
  }, [status]);

  const label =
    transient === 'checking' ? 'Checking…'
    : transient === 'up-to-date' ? 'Up to date'
    : transient === 'failed' ? "Couldn't check"
    : 'Check for updates';

  const isChecking = transient === 'checking';

  const onClick = () => {
    if (isChecking) return;
    invoke('check_for_updates_now').catch(() => {/* error arrives via event */});
  };

  return (
    <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)] select-none">
      v{__APP_VERSION__}{' · '}
      <button
        type="button"
        onClick={onClick}
        disabled={isChecking}
        className="underline-offset-2 hover:underline hover:text-[color:var(--color-accent)] transition-colors disabled:opacity-60"
      >
        {label}
      </button>
    </span>
  );
}


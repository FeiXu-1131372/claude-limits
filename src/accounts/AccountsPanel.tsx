import { useMemo, useState } from 'react';
import { useAppStore } from '../lib/store';
import { ipc } from '../lib/ipc';
import { AccountRow } from './AccountRow';
import { AddAccountChooser } from './AddAccountChooser';
import { SwapConfirmCard } from './SwapConfirmCard';
import type { AccountListEntry, RunningClaudeCode } from '../lib/generated/bindings';

interface Props {
  onBack: () => void;
}

interface PendingSwap {
  target: AccountListEntry;
  running: RunningClaudeCode;
}

export function AccountsPanel({ onBack }: Props) {
  const accounts = useAppStore((s) => s.accounts);
  const thresholds = useAppStore((s) => (s.settings?.thresholds ?? [75, 90]) as [number, number]);
  const refreshAccounts = useAppStore((s) => s.refreshAccounts);
  const setPendingSwapReport = useAppStore((s) => s.setPendingSwapReport);
  const [chooserOpen, setChooserOpen] = useState(false);
  const [swappingSlot, setSwappingSlot] = useState<number | null>(null);
  const [pending, setPending] = useState<PendingSwap | null>(null);
  const [confirmError, setConfirmError] = useState<string | null>(null);

  const orgGroups = useMemo(() => {
    const map = new Map<string, AccountListEntry>();
    for (const a of accounts) {
      if (a.org_uuid && !map.has(a.org_uuid)) {
        map.set(a.org_uuid, a);
      }
    }
    return map;
  }, [accounts]);

  const currentActive = useMemo(
    () => accounts.find((a) => a.is_active) ?? null,
    [accounts],
  );

  async function requestSwap(entry: AccountListEntry) {
    if (entry.is_active || swappingSlot !== null) return;
    setConfirmError(null);
    let running: RunningClaudeCode = { cli_processes: 0, vscode_with_extension: [] };
    try {
      running = await ipc.detectRunningClaudeCode();
    } catch {
      // Detection is best-effort — fall through with empty running state.
    }
    setPending({ target: entry, running });
  }

  async function confirmSwap() {
    if (!pending || swappingSlot !== null) return;
    setConfirmError(null);
    setSwappingSlot(pending.target.slot);
    try {
      const report = await ipc.swapToAccount(pending.target.slot);
      setPendingSwapReport(report);
      await refreshAccounts();
      setPending(null);
    } catch (e) {
      setConfirmError(e instanceof Error ? e.message : 'Swap failed');
    } finally {
      setSwappingSlot(null);
    }
  }

  if (chooserOpen) {
    return <AddAccountChooser onClose={() => setChooserOpen(false)} />;
  }

  if (pending) {
    return (
      <SwapConfirmCard
        current={currentActive}
        target={pending.target}
        running={pending.running}
        busy={swappingSlot !== null}
        errorMessage={confirmError}
        onConfirm={confirmSwap}
        onCancel={() => {
          setPending(null);
          setConfirmError(null);
        }}
      />
    );
  }

  return (
    <div className="flex h-full w-full flex-col">
      <div className="flex items-center justify-between px-[var(--popover-pad)] pt-[var(--space-md)] pb-[var(--space-sm)]">
        <button
          type="button"
          onClick={onBack}
          className="text-[length:var(--text-label)] text-[color:var(--color-text-secondary)] hover:text-[color:var(--color-text)]"
        >
          ← Back
        </button>
        <span className="text-[length:var(--text-label)] uppercase tracking-[var(--tracking-label)] text-[color:var(--color-text-secondary)]">
          Accounts
        </span>
        <span style={{ width: '24px' }} />
      </div>

      <div className="flex-1 overflow-y-auto">
        {accounts.length === 0 && (
          <div className="px-[var(--popover-pad)] py-[var(--space-md)] text-[color:var(--color-text-muted)]">
            No accounts managed yet.
          </div>
        )}
        {accounts.map((a) => {
          const groupHead = a.org_uuid ? orgGroups.get(a.org_uuid) : undefined;
          const shareHint =
            groupHead && groupHead.slot !== a.slot ? groupHead.email : null;
          return (
            <AccountRow
              key={a.slot}
              entry={a}
              thresholds={thresholds}
              shareHint={shareHint}
              onSwap={() => requestSwap(a)}
              swapBusy={swappingSlot !== null}
              swapping={swappingSlot === a.slot}
            />
          );
        })}

        <div className="px-[var(--popover-pad)] py-[var(--space-md)]">
          <button
            type="button"
            onClick={() => setChooserOpen(true)}
            className="text-[length:var(--text-label)] text-[color:var(--color-accent)] hover:underline"
          >
            + Add account
          </button>
        </div>
      </div>
    </div>
  );
}

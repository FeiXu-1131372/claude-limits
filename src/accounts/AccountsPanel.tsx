import { useMemo, useState } from 'react';
import { useAppStore } from '../lib/store';
import { ipc } from '../lib/ipc';
import { AccountRow } from './AccountRow';
import { AddAccountChooser } from './AddAccountChooser';
import { SwapConfirmModal } from './SwapConfirmModal';
import type { AccountListEntry, RunningClaudeCode } from '../lib/generated/bindings';

interface Props {
  onBack: () => void;
}

export function AccountsPanel({ onBack }: Props) {
  const accounts = useAppStore((s) => s.accounts);
  const thresholds = useAppStore((s) => (s.settings?.thresholds ?? [75, 90]) as [number, number]);
  const refreshAccounts = useAppStore((s) => s.refreshAccounts);
  const [chooserOpen, setChooserOpen] = useState(false);
  const [confirm, setConfirm] = useState<
    { entry: AccountListEntry; running: RunningClaudeCode } | null
  >(null);
  const [error, setError] = useState<string | null>(null);

  const orgGroups = useMemo(() => {
    const map = new Map<string, AccountListEntry>();
    for (const a of accounts) {
      if (a.org_uuid && !map.has(a.org_uuid)) {
        map.set(a.org_uuid, a);
      }
    }
    return map;
  }, [accounts]);

  async function tryRowSwap(entry: AccountListEntry) {
    setError(null);
    if (entry.is_active) return;
    const running = await ipc.detectRunningClaudeCode();
    if (running.cli_processes === 0 && running.vscode_with_extension.length === 0) {
      await performSwap(entry);
    } else {
      setConfirm({ entry, running });
    }
  }

  async function performSwap(entry: AccountListEntry) {
    try {
      await ipc.swapToAccount(entry.slot);
      await refreshAccounts();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Swap failed');
    } finally {
      setConfirm(null);
    }
  }

  if (chooserOpen) {
    return <AddAccountChooser onClose={() => setChooserOpen(false)} />;
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
              onClick={() => tryRowSwap(a)}
            />
          );
        })}

        {confirm && (
          <div className="px-[var(--popover-pad)] py-[var(--space-sm)]">
            <SwapConfirmModal
              email={confirm.entry.email}
              running={confirm.running}
              onConfirm={() => performSwap(confirm.entry)}
              onCancel={() => setConfirm(null)}
            />
          </div>
        )}

        {error && (
          <span className="block px-[var(--popover-pad)] py-[var(--space-sm)] text-[length:var(--text-micro)] text-[color:var(--color-danger)]">
            {error}
          </span>
        )}

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

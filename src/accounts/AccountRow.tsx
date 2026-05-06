import { useMemo } from 'react';
import { UsageBar } from '../popover/UsageBar';
import { ResetCountdown } from '../popover/ResetCountdown';
import type { AccountListEntry } from '../lib/generated/bindings';

interface Props {
  entry: AccountListEntry;
  thresholds: [number, number];
  shareHint?: string | null;
  onClick?: () => void;
  onMenuOpen?: () => void;
}

function chipText(entry: AccountListEntry): string {
  const tag = entry.org_name ?? 'personal';
  return entry.subscription_type ? `${tag} · ${entry.subscription_type}` : tag;
}

export function AccountRow({ entry, thresholds, shareHint, onClick, onMenuOpen }: Props) {
  const cached = entry.cached_usage;
  const fiveHour = cached?.snapshot.five_hour ?? null;
  const sevenDay = cached?.snapshot.seven_day ?? null;

  const errLabel = useMemo(() => {
    if (entry.last_error === 'auth_required')
      return 'token expired — re-authenticate';
    if (entry.last_error) return 'usage unavailable';
    return null;
  }, [entry.last_error]);

  return (
    <div
      role={onClick && !entry.is_active ? 'button' : undefined}
      tabIndex={onClick && !entry.is_active ? 0 : undefined}
      onClick={onClick}
      className={`
        flex flex-col gap-[var(--space-2xs)] px-[var(--popover-pad)] py-[var(--space-sm)]
        ${onClick && !entry.is_active ? 'cursor-pointer hover:bg-[var(--color-track)]' : ''}
      `}
    >
      <div className="flex items-center gap-[var(--space-xs)]">
        <span
          className={`inline-block h-[6px] w-[6px] rounded-full ${
            entry.is_active ? '' : 'opacity-0'
          }`}
          style={{ background: 'var(--color-accent)' }}
          aria-hidden
        />
        <span className="flex-1 text-[length:var(--text-body)] text-[color:var(--color-text)] truncate">
          {entry.email}
        </span>
        <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
          [{chipText(entry)}]
        </span>
        <button
          type="button"
          aria-label="Account menu"
          className="text-[color:var(--color-text-muted)] hover:text-[color:var(--color-text)] px-[var(--space-2xs)]"
          onClick={(e) => {
            e.stopPropagation();
            onMenuOpen?.();
          }}
        >
          ⋯
        </button>
      </div>

      {errLabel ? (
        <span className="pl-[14px] text-[length:var(--text-micro)] text-[color:var(--color-warn)]">
          └ {errLabel}
        </span>
      ) : (
        <div className="pl-[14px] flex flex-col gap-[var(--space-2xs)]">
          {fiveHour && (
            <div className="flex items-center gap-[var(--space-sm)]">
              <span className="w-[20px] text-[length:var(--text-micro)] text-[color:var(--color-text-muted)] mono">
                5h
              </span>
              <UsageBar value={fiveHour.utilization} warnAt={thresholds[0]} dangerAt={thresholds[1]} compact />
              <span className="w-[36px] text-[length:var(--text-micro)] mono text-right">
                {Math.round(fiveHour.utilization)}%
              </span>
              {fiveHour.resets_at && <ResetCountdown resetsAt={fiveHour.resets_at} compact />}
            </div>
          )}
          {sevenDay && (
            <div className="flex items-center gap-[var(--space-sm)]">
              <span className="w-[20px] text-[length:var(--text-micro)] text-[color:var(--color-text-muted)] mono">
                7d
              </span>
              <UsageBar value={sevenDay.utilization} warnAt={thresholds[0]} dangerAt={thresholds[1]} compact />
              <span className="w-[36px] text-[length:var(--text-micro)] mono text-right">
                {Math.round(sevenDay.utilization)}%
              </span>
              {sevenDay.resets_at && <ResetCountdown resetsAt={sevenDay.resets_at} compact />}
            </div>
          )}
          {shareHint && (
            <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
              └ shares quota with {shareHint}
            </span>
          )}
        </div>
      )}
    </div>
  );
}

import type { RunningClaudeCode } from '../lib/generated/bindings';

interface Props {
  email: string;
  running: RunningClaudeCode;
  onConfirm: () => void;
  onCancel: () => void;
}

export function SwapConfirmModal({ email, running, onConfirm, onCancel }: Props) {
  const hasAny = running.cli_processes > 0 || running.vscode_with_extension.length > 0;
  return (
    <div className="flex flex-col gap-[var(--space-sm)] rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-[var(--popover-pad)] py-[var(--space-sm)]">
      <span className="text-[length:var(--text-body)]">Switch to {email}?</span>
      {hasAny && (
        <>
          <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
            Upstream is running:
          </span>
          <ul className="pl-[var(--space-sm)] text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
            {running.cli_processes > 0 && (
              <li>• CLI · {running.cli_processes} process{running.cli_processes > 1 ? 'es' : ''}</li>
            )}
            {running.vscode_with_extension.map((w) => (
              <li key={w}>• VS Code · {w}</li>
            ))}
          </ul>
          <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
            Sessions will pick up the new account on their next token refresh (~5 min).
            Restart for an immediate switch.
          </span>
        </>
      )}
      <div className="flex justify-end gap-[var(--space-sm)]">
        <button
          type="button"
          onClick={onCancel}
          className="text-[length:var(--text-label)] text-[color:var(--color-text-muted)] hover:text-[color:var(--color-text)]"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={onConfirm}
          className="text-[length:var(--text-label)] text-[color:var(--color-accent)] hover:opacity-80"
        >
          Switch
        </button>
      </div>
    </div>
  );
}

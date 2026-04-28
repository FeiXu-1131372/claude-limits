import { Button } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { IconButton } from '../components/ui/IconButton';
import { X } from '../lib/icons';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { handleDragStart, closeWindow } from '../lib/window-chrome';

export function AuthConflictChooser() {
  const conflict = useAppStore((s) => s.conflict);
  const dismiss = useAppStore((s) => s.dismissBanner);
  if (!conflict) return null;

  async function pick(source: 'OAuth' | 'ClaudeCode') {
    await ipc.pickAuthSource(source);
    dismiss('conflict');
  }

  return (
    <div className="relative flex flex-col h-full">
      <div
        onPointerDown={handleDragStart}
        className="flex items-center justify-end gap-[var(--space-sm)] px-[var(--popover-pad)] pt-[var(--space-md)] pb-[var(--space-sm)] cursor-default select-none"
      >
        <IconButton label="Close" onClick={closeWindow}>
          <X size={13} />
        </IconButton>
      </div>
      <div className="flex items-center justify-center flex-1 px-[var(--space-2xl)] pb-[var(--space-2xl)]">
      <Card className="max-w-sm p-[var(--space-lg)]">
        <div className="flex flex-col gap-[var(--space-md)]">
          <h2 className="text-[length:var(--text-title)] font-[var(--weight-semibold)] text-[color:var(--color-text)]">
            Two Claude accounts detected
          </h2>
          <p className="text-[length:var(--text-label)] text-[color:var(--color-text-muted)]">
            Which one should this app monitor?
          </p>
          <div className="flex flex-col gap-[var(--space-sm)]">
            <Button variant="primary" onClick={() => pick('OAuth')}>
              {conflict.oauth_email}{' '}
              <span className="opacity-60">(signed in to this app)</span>
            </Button>
            <Button variant="ghost" onClick={() => pick('ClaudeCode')}>
              {conflict.cli_email}{' '}
              <span className="opacity-60">(Claude Code)</span>
            </Button>
          </div>
        </div>
      </Card>
    </div>
    </div>
  );
}

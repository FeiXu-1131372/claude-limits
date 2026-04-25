import { Button } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';

export function AuthConflictChooser() {
  const conflict = useAppStore((s) => s.conflict);
  const dismiss = useAppStore((s) => s.dismissBanner);
  if (!conflict) return null;

  async function pick(source: 'OAuth' | 'ClaudeCode') {
    await ipc.pickAuthSource(source);
    dismiss('conflict');
  }

  return (
    <div className="flex items-center justify-center h-full p-[var(--space-2xl)]">
      <Card className="max-w-sm p-[var(--space-lg)]">
        <div className="flex flex-col gap-[var(--space-md)]">
          <h2 className="text-[var(--text-title)] font-[var(--weight-semibold)] text-[var(--color-text)]">
            Two Claude accounts detected
          </h2>
          <p className="text-[var(--text-label)] text-[var(--color-text-muted)]">
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
  );
}

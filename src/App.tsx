import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { CompactPopover } from './popover/CompactPopover';
import { ExpandedReport } from './report/ExpandedReport';
import { AuthPanel } from './settings/AuthPanel';
import { AuthConflictChooser } from './settings/AuthConflictChooser';
import { useAppStore } from './lib/store';
import './styles/globals.css';
import './styles/tokens.css';

export function App() {
  const init = useAppStore((s) => s.init);
  const usage = useAppStore((s) => s.usage);
  const authRequired = useAppStore((s) => s.authRequired);
  const conflict = useAppStore((s) => s.conflict);
  const [initialized, setInitialized] = useState(false);

  useEffect(() => {
    init().finally(() => setInitialized(true));
  }, [init]);

  const label = getCurrentWindow().label;

  // Tag the body so CSS can render the popover with a transparent backdrop
  // (so OS vibrancy shows through) while opaque windows like the report keep
  // a solid surface.
  useEffect(() => {
    document.body.dataset.window = label;
    return () => { delete document.body.dataset.window; };
  }, [label]);

  if (!initialized) {
    return (
      <div className="flex h-full w-full items-center justify-center p-6">
        <span className="text-[var(--color-text-muted)]">Loading…</span>
      </div>
    );
  }

  if (conflict) {
    return <AuthConflictChooser />;
  }

  // Only redirect to AuthPanel for an *explicit* auth-required signal, or when
  // the first fetch returned no usage AND we have no Claude Code creds to fall
  // back on. The initial null-usage state is handled by `initialized` above.
  if (authRequired || !usage) {
    return <AuthPanel />;
  }

  if (label === 'report') {
    return <ExpandedReport />;
  }

  return <CompactPopover />;
}

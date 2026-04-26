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
        <span className="text-[color:var(--color-text-muted)]">Loading…</span>
      </div>
    );
  }

  if (conflict) {
    return <AuthConflictChooser />;
  }

  // Only redirect to AuthPanel on an explicit auth-required signal from the
  // backend. A null `usage` just means the first poll hasn't landed yet —
  // CompactPopover handles that with a Loading state. Conflating "loading"
  // with "auth needed" caused every cold start to flash the sign-in screen.
  if (authRequired) {
    return <AuthPanel />;
  }

  if (label === 'report') {
    return <ExpandedReport />;
  }

  return <CompactPopover />;
}

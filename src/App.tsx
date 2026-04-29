import { useEffect, useState } from 'react';
import { CompactPopover } from './popover/CompactPopover';
import { ExpandedReport } from './report/ExpandedReport';
import { AuthPanel } from './settings/AuthPanel';
import { AuthConflictChooser } from './settings/AuthConflictChooser';
import { useAppStore } from './lib/store';
import { attachUpdateListeners } from './lib/updateEvents';
import './styles/globals.css';
import './styles/tokens.css';

export function App() {
  const init = useAppStore((s) => s.init);
  const authRequired = useAppStore((s) => s.authRequired);
  const conflict = useAppStore((s) => s.conflict);
  const viewMode = useAppStore((s) => s.viewMode);
  const [initialized, setInitialized] = useState(false);

  useEffect(() => {
    init().finally(() => setInitialized(true));
  }, [init]);

  useEffect(() => {
    let teardown: (() => void) | null = null;
    attachUpdateListeners().then((unlisten) => { teardown = unlisten; });
    return () => { teardown?.(); };
  }, []);

  // Tag the body so CSS can differentiate compact (transparent vibrancy)
  // from expanded (solid opaque background).
  useEffect(() => {
    document.body.dataset.viewMode = viewMode;
    if (navigator.userAgent.includes('Windows')) {
      document.documentElement.style.setProperty('--window-radius', '18px');
    }
    return () => { delete document.body.dataset.viewMode; };
  }, [viewMode]);

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

  if (viewMode === 'expanded') {
    return <ExpandedReport />;
  }

  return <CompactPopover />;
}

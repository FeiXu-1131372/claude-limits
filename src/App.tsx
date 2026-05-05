import { useEffect, useState } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { CompactPopover } from './popover/CompactPopover';
import { ExpandedReport } from './report/ExpandedReport';
import { AuthPanel } from './settings/AuthPanel';
import { useAppStore } from './lib/store';
import { attachUpdateListeners } from './lib/updateEvents';
import './styles/globals.css';
import './styles/tokens.css';

export function App() {
  const init = useAppStore((s) => s.init);
  const requiresSetup = useAppStore((s) => s.requiresSetup);
  const accounts = useAppStore((s) => s.accounts);
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

  if (requiresSetup && accounts.length === 0) {
    return <AuthPanel />;
  }

  return (
    <AnimatePresence mode="wait" initial={false}>
      {viewMode === 'expanded' ? (
        <motion.div
          key="expanded"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.14, ease: [0.16, 1, 0.3, 1] }}
          style={{ height: '100%' }}
        >
          <ExpandedReport />
        </motion.div>
      ) : (
        <motion.div
          key="compact"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.14, ease: [0.16, 1, 0.3, 1] }}
          style={{ height: '100%' }}
        >
          <CompactPopover />
        </motion.div>
      )}
    </AnimatePresence>
  );
}

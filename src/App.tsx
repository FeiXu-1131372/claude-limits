import { useEffect, useState } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { CompactPopover } from './popover/CompactPopover';
import { ExpandedReport } from './report/ExpandedReport';
import { SettingsPanel } from './settings/SettingsPanel';
import { AuthPanel } from './settings/AuthPanel';
import { useAppStore } from './lib/store';

type View = 'popover' | 'report' | 'settings';

export function App() {
  const init = useAppStore((s) => s.init);
  const usage = useAppStore((s) => s.usage);
  const authRequired = useAppStore((s) => s.authRequired);
  const [view] = useState<View>('popover');

  useEffect(() => {
    init();
  }, [init]);

  const currentView: View | 'auth' =
    !usage || authRequired ? 'auth' : view;

  return (
    <AnimatePresence mode="wait">
      <motion.div
        key={currentView}
        className="h-full w-full"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={{ duration: 0.15 }}
      >
        {currentView === 'auth' && <AuthPanel />}
        {currentView === 'popover' && <CompactPopover />}
        {currentView === 'report' && <ExpandedReport />}
        {currentView === 'settings' && (
          <div className="h-full p-[var(--space-lg)] overflow-y-auto">
            <SettingsPanel />
          </div>
        )}
      </motion.div>
    </AnimatePresence>
  );
}

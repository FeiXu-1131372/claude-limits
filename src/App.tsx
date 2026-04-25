import { useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { CompactPopover } from './popover/CompactPopover';
import { ExpandedReport } from './report/ExpandedReport';
import { AuthPanel } from './settings/AuthPanel';
import { useAppStore } from './lib/store';
import './styles/globals.css';
import './styles/tokens.css';

export function App() {
  const init = useAppStore((s) => s.init);
  const usage = useAppStore((s) => s.usage);
  const authRequired = useAppStore((s) => s.authRequired);

  useEffect(() => {
    init();
  }, [init]);

  const label = getCurrentWindow().label;

  if (authRequired || !usage) {
    return <AuthPanel />;
  }

  if (label === 'report') {
    return <ExpandedReport />;
  }

  return <CompactPopover />;
}

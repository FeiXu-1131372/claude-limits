import { create } from 'zustand';
import { ipc } from './ipc';
import { subscribe, type AppEvent } from './events';
import type { CachedUsage, Settings } from './types';

interface AppStore {
  usage: CachedUsage | null;
  settings: Settings | null;
  hasClaudeCodeCreds: boolean;
  authRequired: boolean;
  conflict: { oauth_email: string; cli_email: string } | null;
  stale: boolean;
  dbReset: boolean;

  init: () => Promise<void>;
  refreshSettings: () => Promise<void>;
  setSettings: (s: Settings) => Promise<void>;
  dismissBanner: (kind: 'authRequired' | 'stale' | 'dbReset' | 'conflict') => void;
}

export const useAppStore = create<AppStore>((set, _get) => ({
  usage: null,
  settings: null,
  hasClaudeCodeCreds: false,
  authRequired: false,
  conflict: null,
  stale: false,
  dbReset: false,

  async init() {
    const [usage, settings, hasClaudeCodeCreds] = await Promise.all([
      ipc.getCurrentUsage(),
      ipc.getSettings(),
      ipc.hasClaudeCodeCreds().catch(() => false),
    ]);
    set({ usage, settings, hasClaudeCodeCreds });

    await subscribe((e: AppEvent) => {
      switch (e.type) {
        case 'usage_updated':
          set({ usage: e.payload, authRequired: false, stale: false });
          break;
        case 'session_ingested':
          break;
        case 'auth_required':
          set({ authRequired: true });
          break;
        case 'auth_source_conflict':
          set({ conflict: e.payload });
          break;
        case 'stale_data':
          set({ stale: true });
          break;
        case 'db_reset':
          set({ dbReset: true });
          break;
      }
    });
  },

  async refreshSettings() {
    const s = await ipc.getSettings();
    set({ settings: s });
  },

  async setSettings(s) {
    await ipc.updateSettings(s);
    set({ settings: s });
  },

  dismissBanner(kind) {
    switch (kind) {
      case 'authRequired':
        set({ authRequired: false });
        break;
      case 'stale':
        set({ stale: false });
        break;
      case 'dbReset':
        set({ dbReset: false });
        break;
      case 'conflict':
        set({ conflict: null });
        break;
    }
  },
}));

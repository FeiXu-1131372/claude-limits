import { create } from 'zustand';
import { getCurrentWindow } from '@tauri-apps/api/window';
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
  // Bumped whenever the backend ingests new JSONL sessions. Tabs that derive
  // their data from session history can subscribe to this so they auto-refresh
  // as Claude Code writes new turns; without it, the report shows the snapshot
  // from when the tab first mounted and only updates on manual reload.
  sessionDataVersion: number;

  init: () => Promise<void>;
  refreshSettings: () => Promise<void>;
  setSettings: (s: Settings) => Promise<void>;
  refreshUsage: () => Promise<void>;
  signOut: () => Promise<void>;
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
  sessionDataVersion: 0,

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
          set((s) => ({ sessionDataVersion: s.sessionDataVersion + 1 }));
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

    // Re-pull cached usage whenever the popover gains focus. Tauri can suspend
    // a hidden webview's JS, so usage_updated events fired between hide/show
    // never reach the store. Without this the popover would happily display
    // stale numbers while the tray title (updated directly from Rust) shows
    // the truth.
    try {
      const win = getCurrentWindow();
      await win.onFocusChanged(({ payload: focused }) => {
        if (!focused) return;
        ipc.getCurrentUsage().then((u) => {
          if (u) set({ usage: u, stale: false });
        }).catch(() => {});
      });
    } catch {
      // Outside Tauri (e.g. localhost demo page) — no focus tracking.
    }
  },

  async refreshSettings() {
    const s = await ipc.getSettings();
    set({ settings: s });
  },

  async setSettings(s) {
    await ipc.updateSettings(s);
    set({ settings: s });
  },

  async refreshUsage() {
    const u = await ipc.getCurrentUsage();
    if (u) set({ usage: u, stale: false });
  },

  async signOut() {
    await ipc.signOut();
    // Rust clears credentials and cached_usage but doesn't push an
    // auth_required event for explicit sign-out. Update the store so
    // App.tsx routes to AuthPanel and the user can pick an auth method.
    const hasClaudeCodeCreds = await ipc.hasClaudeCodeCreds().catch(() => false);
    set({ usage: null, authRequired: true, conflict: null, hasClaudeCodeCreds });
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

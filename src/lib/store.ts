import { create } from 'zustand';

/* ─── Types matching the Rust IPC surface ─── */

export interface Utilization {
  used_pct: number;
  reset_at: string; // ISO 8601
}

export type Model = 'opus' | 'sonnet' | 'haiku';

export interface UsageSnapshot {
  five_hour: Utilization;
  seven_day: Utilization;
  per_model: Record<Model, Utilization>;
  fetched_at: string;
  is_stale: boolean;
}

export interface Settings {
  poll_interval_min: number;
  warn_threshold: number;
  danger_threshold: number;
  launch_at_login: boolean;
  theme: 'system' | 'light' | 'dark';
}

/* ─── Placeholder data for preview ─── */

const PLACEHOLDER_SNAPSHOT: UsageSnapshot = {
  five_hour: { used_pct: 68, reset_at: new Date(Date.now() + 6120000).toISOString() },
  seven_day: { used_pct: 81, reset_at: new Date(Date.now() + 302400000).toISOString() },
  per_model: {
    opus: { used_pct: 42, reset_at: '' },
    sonnet: { used_pct: 31, reset_at: '' },
    haiku: { used_pct: 8, reset_at: '' },
  },
  fetched_at: new Date(Date.now() - 120000).toISOString(),
  is_stale: false,
};

const PLACEHOLDER_SETTINGS: Settings = {
  poll_interval_min: 5,
  warn_threshold: 75,
  danger_threshold: 90,
  launch_at_login: true,
  theme: 'system',
};

/* ─── Store ─── */

interface AppStore {
  snapshot: UsageSnapshot | null;
  settings: Settings;
  authState: 'authenticated' | 'unauthenticated' | 'loading';
  showSettings: boolean;

  setSnapshot: (s: UsageSnapshot) => void;
  setSettings: (s: Settings) => void;
  setAuthState: (s: AppStore['authState']) => void;
  setShowSettings: (show: boolean) => void;
}

export const useStore = create<AppStore>((set) => ({
  snapshot: PLACEHOLDER_SNAPSHOT,
  settings: PLACEHOLDER_SETTINGS,
  authState: 'authenticated',
  showSettings: false,

  setSnapshot: (snapshot) => set({ snapshot }),
  setSettings: (settings) => set({ settings }),
  setAuthState: (authState) => set({ authState }),
  setShowSettings: (showSettings) => set({ showSettings }),
}));

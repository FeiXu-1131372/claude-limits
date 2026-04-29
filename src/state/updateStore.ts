import { create } from 'zustand';

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'up-to-date'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'failed';

export type UpdatePhase = 'check' | 'download' | 'verify' | 'install';

export interface UpdateError {
  phase: UpdatePhase;
  message: string;
}

interface UpdateState {
  status: UpdateStatus;
  version: string | null;
  progress: number;
  error: UpdateError | null;
  lastCheckedAt: string | null;

  setStatus: (s: UpdateStatus) => void;
  setUpToDate: (checkedAt: string) => void;
  setAvailable: (version: string) => void;
  setProgress: (progress: number) => void;
  setReady: (version: string) => void;
  setFailed: (phase: UpdatePhase, message: string) => void;
  reset: () => void;
}

export const useUpdateStore = create<UpdateState>((set) => ({
  status: 'idle',
  version: null,
  progress: 0,
  error: null,
  lastCheckedAt: null,

  setStatus: (status) => set({ status }),
  setUpToDate: (lastCheckedAt) => set({ status: 'up-to-date', lastCheckedAt, error: null }),
  setAvailable: (version) => set({ status: 'available', version, error: null }),
  setProgress: (progress) => set({ status: 'downloading', progress }),
  setReady: (version) => set({ status: 'ready', version, progress: 1, error: null }),
  setFailed: (phase, message) => set({ status: 'failed', error: { phase, message } }),
  reset: () =>
    set({ status: 'idle', version: null, progress: 0, error: null }),
}));

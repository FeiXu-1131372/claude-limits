import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useUpdateStore, type UpdatePhase } from '../state/updateStore';

interface AvailablePayload { version: string; notes?: string; pubDate?: string }
interface UpToDatePayload { checkedAt: string }
interface ProgressPayload { downloaded: number; total: number }
interface ReadyPayload { version: string }
interface FailedPayload { phase: UpdatePhase; message: string }

/**
 * Attach all updater event listeners. Call once at app startup.
 * Returns a teardown function that unregisters every listener.
 */
export async function attachUpdateListeners(): Promise<UnlistenFn> {
  const store = useUpdateStore.getState();

  const unlisteners = await Promise.all([
    listen('update://checking', () => store.setStatus('checking')),
    listen<UpToDatePayload>('update://up-to-date', (e) => store.setUpToDate(e.payload.checkedAt)),
    listen<AvailablePayload>('update://available', (e) => store.setAvailable(e.payload.version)),
    listen<ProgressPayload>('update://progress', (e) => {
      const total = e.payload.total || 1;
      store.setProgress(Math.min(1, e.payload.downloaded / total));
    }),
    listen<ReadyPayload>('update://ready', (e) => store.setReady(e.payload.version)),
    listen<FailedPayload>('update://failed', (e) =>
      store.setFailed(e.payload.phase, e.payload.message),
    ),
  ]);

  return () => unlisteners.forEach((u) => u());
}

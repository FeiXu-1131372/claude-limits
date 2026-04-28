import { useCallback, useEffect, useRef, useState } from 'react';

function toMessage(e: unknown): string {
  if (e instanceof Error && e.message) return e.message;
  if (typeof e === 'string' && e.length > 0) return e;
  if (e && typeof e === 'object' && 'message' in e && typeof (e as { message: unknown }).message === 'string') {
    return (e as { message: string }).message;
  }
  return 'Unknown error';
}

export interface TabDataState<T> {
  data: T | null;
  error: string | null;
  loading: boolean;
  reload: () => void;
}

/**
 * Loads tab data with consistent loading/error/reload semantics. The loader
 * runs on mount, on explicit reload(), and whenever any value in `triggers`
 * changes — pass a session-data-version counter here so the tab auto-refreshes
 * as new data arrives in the background.
 */
export function useTabData<T>(
  loader: () => Promise<T>,
  triggers: ReadonlyArray<unknown> = [],
): TabDataState<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [tick, setTick] = useState(0);
  // Track whether the first fetch has completed so trigger-driven refreshes
  // can repaint without flickering "Loading…" between snapshots.
  const hasLoadedOnce = useRef(false);

  useEffect(() => {
    let cancelled = false;
    if (!hasLoadedOnce.current) setLoading(true);
    setError(null);
    loader()
      .then((v) => {
        if (cancelled) return;
        setData(v);
        hasLoadedOnce.current = true;
      })
      .catch((e) => {
        if (cancelled) return;
        setError(toMessage(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
    // The loader closure is recomputed on every render; we deliberately
    // re-run only on explicit reload() (tick) or when an external trigger fires.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tick, ...triggers]);

  const reload = useCallback(() => setTick((t) => t + 1), []);
  return { data, error, loading, reload };
}

import { useCallback, useEffect, useState } from 'react';

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
 * function is called on mount and on every reload(). The hook tracks whether
 * the load is in flight so consumers can render Loading / Error / Empty
 * states uniformly.
 */
export function useTabData<T>(loader: () => Promise<T>): TabDataState<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    loader()
      .then((v) => {
        if (cancelled) return;
        setData(v);
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
    // re-run only on explicit reload() (tick).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tick]);

  const reload = useCallback(() => setTick((t) => t + 1), []);
  return { data, error, loading, reload };
}

import { useEffect, useState } from 'react';

function humanize(ms: number): string {
  if (ms <= 0) return 'now';
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

/**
 * Compact reset caption — sans "in" prefix in the system font + the duration
 * in mono. Splitting the family at the word/number boundary is the design's
 * core typographic rule: numbers are JetBrains Mono, copy is system sans.
 */
export function ResetCountdown({ resetsAt }: { resetsAt: string }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const i = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(i);
  }, []);
  const target = new Date(resetsAt).getTime();
  return (
    <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)] whitespace-nowrap">
      in{' '}
      <span
        className="tabular-nums"
        style={{ fontFamily: 'var(--font-mono)' }}
      >
        {humanize(target - now)}
      </span>
    </span>
  );
}

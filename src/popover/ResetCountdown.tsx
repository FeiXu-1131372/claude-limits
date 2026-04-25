import { useEffect, useState } from "react";

function humanize(ms: number): string {
  if (ms <= 0) return "now";
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export function ResetCountdown({ resetsAt }: { resetsAt: string }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const i = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(i);
  }, []);
  const target = new Date(resetsAt).getTime();
  return (
    <span className="mono text-[var(--text-micro)] text-[var(--color-text-muted)]">
      Resets in {humanize(target - now)}
    </span>
  );
}

import { useEffect, useState } from 'react';
import { Card } from '../components/ui/Card';
import { Toggle } from '../components/ui/Toggle';
import { Slider } from '../components/ui/Slider';
import { Select } from '../components/ui/Select';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { useAppStore } from '../lib/store';
import { ipc } from '../lib/ipc';
import { LogOut } from '../lib/icons';
import type { Settings } from '../lib/types';
import { enable as enableAutostart, disable as disableAutostart } from '@tauri-apps/plugin-autostart';

export function SettingsPanel() {
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);
  const [local, setLocal] = useState<Settings | null>(settings);

  useEffect(() => setLocal(settings), [settings]);

  if (!local) return <p className="text-[var(--color-text-muted)]">Loading...</p>;

  const clamp = (n: number, min: number, max: number) => Math.min(max, Math.max(min, n));
  const pollingMinutes = Math.round(local.polling_interval_secs / 60);

  async function save() {
    const next: Settings = { ...local!, polling_interval_secs: clamp(local!.polling_interval_secs, 60, 1800) };
    await setSettings(next);
    try {
      if (next.launch_at_login) await enableAutostart();
      else await disableAutostart();
    } catch (e) {
      console.warn('autostart toggle failed', e);
    }
  }

  async function signOut() {
    await ipc.signOut();
  }

  return (
    <div className="flex flex-col gap-[var(--space-lg)] h-full overflow-y-auto">
      {/* General */}
      <section className="flex flex-col gap-[var(--space-sm)]">
        <h2 className="text-[var(--text-label)] font-[var(--weight-semibold)] text-[var(--color-text-muted)] uppercase tracking-[0.04em] px-[var(--space-2xs)]">
          General
        </h2>
        <Card className="p-[var(--space-md)] flex flex-col">
          <Toggle
            label="Launch at login"
            description="Start monitoring when you log in"
            defaultChecked={local.launch_at_login}
          />
          <Select
            label="Theme"
            options={[
              { value: 'system', label: 'System' },
              { value: 'light', label: 'Light' },
              { value: 'dark', label: 'Dark' },
            ]}
            defaultValue={local.theme}
          />
        </Card>
      </section>

      {/* Polling */}
      <section className="flex flex-col gap-[var(--space-sm)]">
        <h2 className="text-[var(--text-label)] font-[var(--weight-semibold)] text-[var(--color-text-muted)] uppercase tracking-[0.04em] px-[var(--space-2xs)]">
          Polling
        </h2>
        <Card className="p-[var(--space-md)]">
          <Slider
            label="Poll interval"
            min={1}
            max={30}
            step={1}
            defaultValue={pollingMinutes}
            formatValue={(v) => `${v}m`}
          />
          {pollingMinutes <= 2 && (
            <p className="text-[var(--text-micro)] text-[var(--color-warn)] mt-[var(--space-xs)]">
              Frequent polling may cause rate limiting
            </p>
          )}
        </Card>
      </section>

      {/* Thresholds */}
      <section className="flex flex-col gap-[var(--space-sm)]">
        <h2 className="text-[var(--text-label)] font-[var(--weight-semibold)] text-[var(--color-text-muted)] uppercase tracking-[0.04em] px-[var(--space-2xs)]">
          Notifications
        </h2>
        <Card className="p-[var(--space-md)] flex flex-col gap-[var(--space-md)]">
          {local.thresholds.map((t, i) => (
            <Slider
              key={i}
              label={`Threshold ${i + 1}`}
              min={25}
              max={95}
              step={5}
              defaultValue={t}
              formatValue={(v) => `${v}%`}
            />
          ))}
          <div className="flex items-center gap-[var(--space-sm)] px-[var(--space-2xs)]">
            <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">
              Notifications fire once per bucket reset cycle
            </span>
          </div>
        </Card>
      </section>

      {/* Account */}
      <section className="flex flex-col gap-[var(--space-sm)]">
        <h2 className="text-[var(--text-label)] font-[var(--weight-semibold)] text-[var(--color-text-muted)] uppercase tracking-[0.04em] px-[var(--space-2xs)]">
          Account
        </h2>
        <Card className="p-[var(--space-md)]">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-[var(--space-sm)]">
              <span className="text-[var(--text-body)] text-[var(--color-text)]">Connected</span>
              <Badge variant="live">OAuth</Badge>
            </div>
            <Button variant="ghost" size="sm" className="text-[var(--color-danger)]" onClick={signOut}>
              <LogOut size={12} />
              Sign out
            </Button>
          </div>
        </Card>
      </section>

      {/* Save */}
      <div className="flex justify-end px-[var(--space-2xs)]">
        <Button variant="primary" onClick={save}>Save</Button>
      </div>

      {/* Spacer */}
      <div className="h-[var(--space-xl)]" />
    </div>
  );
}

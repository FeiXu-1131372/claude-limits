import { Card } from '../components/ui/Card';
import { Toggle } from '../components/ui/Toggle';
import { Slider } from '../components/ui/Slider';
import { Select } from '../components/ui/Select';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { useStore } from '../lib/store';
import { IconRefresh, IconLogOut } from '../lib/icons';

export function SettingsPanel() {
  const settings = useStore((s) => s.settings);

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
            defaultChecked={settings.launch_at_login}
          />
          <Select
            label="Theme"
            options={[
              { value: 'system', label: 'System' },
              { value: 'light', label: 'Light' },
              { value: 'dark', label: 'Dark' },
            ]}
            defaultValue={settings.theme}
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
            defaultValue={settings.poll_interval_min}
            formatValue={(v) => `${v}m`}
          />
          {settings.poll_interval_min <= 2 && (
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
          <Slider
            label="Warning threshold"
            min={50}
            max={89}
            step={1}
            defaultValue={settings.warn_threshold}
            formatValue={(v) => `${v}%`}
          />
          <Slider
            label="Danger threshold"
            min={51}
            max={99}
            step={1}
            defaultValue={settings.danger_threshold}
            formatValue={(v) => `${v}%`}
          />
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
            <Button variant="ghost" size="sm" className="text-[var(--color-danger)]">
              <IconLogOut size={12} />
              Sign out
            </Button>
          </div>
        </Card>
      </section>

      {/* Spacer to prevent content from being cut off */}
      <div className="h-[var(--space-xl)]" />
    </div>
  );
}

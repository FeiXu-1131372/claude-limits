import { useState } from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { motion } from 'framer-motion';
import { Card } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { fadeIn } from '../lib/motion';
import { IconAuth, IconRefresh, ExternalLink } from '../lib/icons';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';

type Step = 'choose' | 'waiting' | 'paste' | 'submitting';

export function AuthPanel() {
  const hasClaudeCodeCreds = useAppStore((s) => s.hasClaudeCodeCreds);
  const [step, setStep] = useState<Step>('choose');
  const [code, setCode] = useState('');
  const [error, setError] = useState<string | null>(null);

  async function startOauth() {
    setError(null);
    try {
      const url = await ipc.startOauthFlow();
      await openUrl(url);
      setStep('paste');
    } catch (e) {
      setError(String(e));
    }
  }

  async function submit() {
    setError(null);
    setStep('submitting');
    try {
      await ipc.submitOauthCode(code.trim());
      setStep('choose');
      setCode('');
    } catch (e) {
      setError(String(e));
      setStep('paste');
    }
  }

  async function useLocal() {
    setError(null);
    try {
      await ipc.useClaudeCodeCreds();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="flex items-center justify-center h-full p-[var(--space-2xl)]">
      <motion.div
        className="flex flex-col gap-[var(--space-xl)] max-w-[280px]"
        variants={fadeIn}
        initial="hidden"
        animate="visible"
      >
        {/* Icon */}
        <div className="flex justify-center">
          <div className="w-[48px] h-[48px] rounded-[var(--radius-lg)] bg-[var(--color-accent-dim)] flex items-center justify-center">
            <IconAuth size={24} className="text-[var(--color-accent)]" />
          </div>
        </div>

        {/* Title */}
        <div className="text-center flex flex-col gap-[var(--space-xs)]">
          <h1 className="text-[var(--text-title)] font-[var(--weight-semibold)] text-[var(--color-text)]">
            Connect to Claude
          </h1>
          <p className="text-[var(--text-label)] text-[var(--color-text-muted)] leading-[var(--leading-label)]">
            {step === 'paste'
              ? 'Paste the code shown on the callback page:'
              : 'Choose how to authenticate. Your credentials stay on this device.'}
          </p>
        </div>

        {step === 'choose' && (
          <div className="flex flex-col gap-[var(--space-sm)]">
            <Card hover className="p-[var(--space-md)]">
              <button
                onClick={startOauth}
                className="w-full flex items-center gap-[var(--space-sm)] text-left"
              >
                <div className="w-[32px] h-[32px] rounded-[var(--radius-sm)] bg-[var(--color-accent-dim)] flex items-center justify-center shrink-0">
                  <ExternalLink size={14} className="text-[var(--color-accent)]" />
                </div>
                <div className="flex flex-col gap-[2px] flex-1">
                  <span className="text-[var(--text-body)] font-[var(--weight-medium)] text-[var(--color-text)]">
                    Sign in with Claude
                  </span>
                  <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">
                    Opens browser for secure OAuth
                  </span>
                </div>
              </button>
            </Card>

            {hasClaudeCodeCreds && (
              <Card hover className="p-[var(--space-md)]">
                <button
                  onClick={useLocal}
                  className="w-full flex items-center gap-[var(--space-sm)] text-left"
                >
                  <div className="w-[32px] h-[32px] rounded-[var(--radius-sm)] bg-[var(--color-track)] flex items-center justify-center shrink-0">
                    <IconAuth size={14} className="text-[var(--color-text-secondary)]" />
                  </div>
                  <div className="flex flex-col gap-[2px] flex-1">
                    <span className="text-[var(--text-body)] font-[var(--weight-medium)] text-[var(--color-text)]">
                      Use Claude Code credentials
                    </span>
                    <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">
                      Reads from your existing session
                    </span>
                  </div>
                </button>
              </Card>
            )}
          </div>
        )}

        {step === 'paste' && (
          <div className="flex flex-col gap-[var(--space-sm)]">
            <input
              autoFocus
              className="rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-[var(--space-sm)] py-[var(--space-xs)] mono text-[var(--text-label)] text-[var(--color-text)] w-full"
              placeholder="code#state"
              value={code}
              onChange={(e) => setCode(e.target.value)}
            />
            <div className="flex justify-end gap-[var(--space-sm)]">
              <Button variant="ghost" onClick={() => setStep('choose')}>
                Cancel
              </Button>
              <Button
                variant="primary"
                onClick={submit}
                disabled={!code.includes('#')}
              >
                Continue
              </Button>
            </div>
          </div>
        )}

        {step === 'submitting' && (
          <div className="flex items-center justify-center gap-[var(--space-sm)]">
            <IconRefresh size={14} className="text-[var(--color-accent)] animate-spin" />
            <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">
              Verifying...
            </span>
          </div>
        )}

        {error && (
          <p className="text-[var(--text-label)] text-[var(--color-danger)]">
            {error}
          </p>
        )}

        {/* Security note */}
        <p className="text-[var(--text-micro)] text-[var(--color-text-muted)] text-center">
          Credentials are stored in your OS keychain and never leave this device.
        </p>
      </motion.div>
    </div>
  );
}

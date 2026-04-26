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

function toMessage(e: unknown, fallback: string): string {
  if (e instanceof Error && e.message) return e.message;
  if (typeof e === 'string' && e.length > 0) return e;
  if (e && typeof e === 'object' && 'message' in e && typeof (e as { message: unknown }).message === 'string') {
    return (e as { message: string }).message;
  }
  return fallback;
}

export function AuthPanel() {
  const hasClaudeCodeCreds = useAppStore((s) => s.hasClaudeCodeCreds);
  const [step, setStep] = useState<Step>('choose');
  const [code, setCode] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [authorizeUrl, setAuthorizeUrl] = useState<string | null>(null);

  async function startOauth() {
    setError(null);
    let url: string;
    try {
      url = await ipc.startOauthFlow();
    } catch (e) {
      setError(toMessage(e, 'Failed to start sign-in.'));
      return;
    }
    setAuthorizeUrl(url);
    setStep('paste');
    try {
      await openUrl(url);
    } catch (e) {
      // Browser open failed — user can still copy the URL we just exposed.
      setError(
        `Could not open your browser (${toMessage(e, 'unknown error')}). Copy the link below and open it manually.`,
      );
    }
  }

  async function submit() {
    setError(null);
    setStep('submitting');
    try {
      await ipc.submitOauthCode(code.trim());
      setStep('choose');
      setCode('');
      setAuthorizeUrl(null);
    } catch (e) {
      setError(toMessage(e, 'Sign-in failed. Try again.'));
      setStep('paste');
    }
  }

  async function useLocal() {
    setError(null);
    try {
      await ipc.useClaudeCodeCreds();
    } catch (e) {
      setError(toMessage(e, 'Failed to use Claude Code credentials.'));
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
            <IconAuth size={24} className="text-[color:var(--color-accent)]" />
          </div>
        </div>

        {/* Title */}
        <div className="text-center flex flex-col gap-[var(--space-xs)]">
          <h1 className="text-[length:var(--text-title)] font-[var(--weight-semibold)] text-[color:var(--color-text)]">
            Connect to Claude
          </h1>
          <p className="text-[length:var(--text-label)] text-[color:var(--color-text-muted)] leading-[var(--leading-label)]">
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
                  <ExternalLink size={14} className="text-[color:var(--color-accent)]" />
                </div>
                <div className="flex flex-col gap-[2px] flex-1">
                  <span className="text-[length:var(--text-body)] font-[var(--weight-medium)] text-[color:var(--color-text)]">
                    Sign in with Claude
                  </span>
                  <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
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
                    <IconAuth size={14} className="text-[color:var(--color-text-secondary)]" />
                  </div>
                  <div className="flex flex-col gap-[2px] flex-1">
                    <span className="text-[length:var(--text-body)] font-[var(--weight-medium)] text-[color:var(--color-text)]">
                      Use Claude Code credentials
                    </span>
                    <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
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
            {authorizeUrl && (
              <div className="flex flex-col gap-[var(--space-2xs)]">
                <span className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)]">
                  Authorize URL (browser should have opened):
                </span>
                <div className="flex items-center gap-[var(--space-2xs)]">
                  <code className="flex-1 truncate rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-[var(--space-xs)] py-[var(--space-2xs)] mono text-[length:var(--text-micro)] text-[color:var(--color-text-secondary)]">
                    {authorizeUrl}
                  </code>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => navigator.clipboard?.writeText(authorizeUrl).catch(() => {})}
                  >
                    Copy
                  </Button>
                </div>
              </div>
            )}
            <input
              autoFocus
              className="rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-[var(--space-sm)] py-[var(--space-xs)] mono text-[length:var(--text-label)] text-[color:var(--color-text)] w-full"
              placeholder="code#state"
              value={code}
              onChange={(e) => {
                setCode(e.target.value);
                if (error) setError(null);
              }}
            />
            <div className="flex justify-end gap-[var(--space-sm)]">
              <Button variant="ghost" onClick={() => { setStep('choose'); setError(null); setAuthorizeUrl(null); }}>
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
            <IconRefresh size={14} className="text-[color:var(--color-accent)] animate-spin" />
            <span className="text-[length:var(--text-label)] text-[color:var(--color-text-muted)]">
              Verifying...
            </span>
          </div>
        )}

        {error && (
          <p className="text-[length:var(--text-label)] text-[color:var(--color-danger)]">
            {error}
          </p>
        )}

        {/* Security note */}
        <p className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)] text-center">
          Credentials are stored in your OS keychain and never leave this device.
        </p>
      </motion.div>
    </div>
  );
}

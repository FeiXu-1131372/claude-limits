import { useState } from 'react';
import { motion } from 'framer-motion';
import { Card } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { IconButton } from '../components/ui/IconButton';
import { fadeIn } from '../lib/motion';
import { IconAuth, IconRefresh, IconExternalLink } from '../lib/icons';

type AuthMethod = 'oauth' | 'local' | null;

export function AuthPanel() {
  const [loading, setLoading] = useState(false);
  const [selectedMethod, setSelectedMethod] = useState<AuthMethod>(null);

  const handleOAuth = () => {
    setSelectedMethod('oauth');
    setLoading(true);
    // In production: invoke('sign_in_with_claude')
    setTimeout(() => setLoading(false), 2000);
  };

  const handleLocalCreds = () => {
    setSelectedMethod('local');
    setLoading(true);
    // In production: invoke('use_claude_code_creds')
    setTimeout(() => setLoading(false), 2000);
  };

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
            Choose how to authenticate. Your credentials stay on this device.
          </p>
        </div>

        {/* Auth options */}
        <div className="flex flex-col gap-[var(--space-sm)]">
          <Card hover className="p-[var(--space-md)]">
            <button
              onClick={handleOAuth}
              disabled={loading}
              className="w-full flex items-center gap-[var(--space-sm)] text-left"
            >
              <div className="w-[32px] h-[32px] rounded-[var(--radius-sm)] bg-[var(--color-accent-dim)] flex items-center justify-center shrink-0">
                <IconExternalLink size={14} className="text-[var(--color-accent)]" />
              </div>
              <div className="flex flex-col gap-[2px] flex-1">
                <span className="text-[var(--text-body)] font-[var(--weight-medium)] text-[var(--color-text)]">
                  Sign in with Claude
                </span>
                <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">
                  Opens browser for secure OAuth
                </span>
              </div>
              {loading && selectedMethod === 'oauth' && (
                <IconRefresh size={14} className="text-[var(--color-accent)] animate-spin" />
              )}
            </button>
          </Card>

          <Card hover className="p-[var(--space-md)]">
            <button
              onClick={handleLocalCreds}
              disabled={loading}
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
              {loading && selectedMethod === 'local' && (
                <IconRefresh size={14} className="text-[var(--color-accent)] animate-spin" />
              )}
            </button>
          </Card>
        </div>

        {/* Security note */}
        <p className="text-[var(--text-micro)] text-[var(--color-text-muted)] text-center">
          Credentials are stored in your OS keychain and never leave this device.
        </p>
      </motion.div>
    </div>
  );
}

import { commands, type Result } from './generated/bindings';
import type { AuthSource, Settings } from './types';

/** Unwrap a specta-generated Result, throwing on error. */
async function unwrap<T>(r: Result<T, string>): Promise<T> {
  if (r.status === 'error') throw new Error(r.error);
  return r.data;
}

export const ipc = {
  getCurrentUsage: () => commands.getCurrentUsage().then(unwrap),
  getSessionHistory: (days: number) => commands.getSessionHistory(days).then(unwrap),
  getDailyTrends: (days: number) => commands.getDailyTrends(days).then(unwrap),
  getModelBreakdown: (days: number) => commands.getModelBreakdown(days).then(unwrap),
  getProjectBreakdown: (days: number) => commands.getProjectBreakdown(days).then(unwrap),
  getCacheStats: (days: number) => commands.getCacheStats(days).then(unwrap),

  startOauthFlow: () => commands.startOauthFlow().then(unwrap),
  submitOauthCode: (pasted: string) => commands.submitOauthCode(pasted).then(unwrap),
  useClaudeCodeCreds: () => commands.useClaudeCodeCreds().then(unwrap),
  pickAuthSource: (source: AuthSource) => commands.pickAuthSource(source).then(unwrap),
  signOut: () => commands.signOut().then(unwrap),
  hasClaudeCodeCreds: () => commands.hasClaudeCodeCreds().then(unwrap),

  getSettings: () => commands.getSettings().then(unwrap),
  updateSettings: (s: Settings) => commands.updateSettings(s).then(unwrap),

  resizeWindow: (mode: 'compact' | 'expanded') => commands.resizeWindow(mode).then(unwrap),
  forceRefresh: () => commands.forceRefresh().then(unwrap),
};

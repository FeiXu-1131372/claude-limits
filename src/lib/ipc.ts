import { invoke } from "@tauri-apps/api/core";
import type {
  AuthSource,
  CachedUsage,
  CacheStats,
  DailyBucket,
  ModelStats,
  ProjectStats,
  SessionEvent,
  Settings,
} from "./types";

export const ipc = {
  getCurrentUsage: () => invoke<CachedUsage | null>("get_current_usage"),
  getSessionHistory: (days: number) =>
    invoke<SessionEvent[]>("get_session_history", { days }),
  getDailyTrends: (days: number) =>
    invoke<DailyBucket[]>("get_daily_trends", { days }),
  getModelBreakdown: (days: number) =>
    invoke<ModelStats[]>("get_model_breakdown", { days }),
  getProjectBreakdown: (days: number) =>
    invoke<ProjectStats[]>("get_project_breakdown", { days }),
  getCacheStats: (days: number) =>
    invoke<CacheStats>("get_cache_stats", { days }),

  startOauthFlow: () => invoke<string>("start_oauth_flow"),
  submitOauthCode: (pasted: string) =>
    invoke<void>("submit_oauth_code", { pasted }),
  useClaudeCodeCreds: () => invoke<void>("use_claude_code_creds"),
  pickAuthSource: (source: AuthSource) =>
    invoke<void>("pick_auth_source", { source }),
  signOut: () => invoke<void>("sign_out"),
  hasClaudeCodeCreds: () => invoke<boolean>("has_claude_code_creds"),

  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (s: Settings) =>
    invoke<void>("update_settings", { s }),
};

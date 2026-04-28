export type AuthSource = "OAuth" | "ClaudeCode";

export interface Utilization {
  utilization: number;
  resets_at: string;
}

export interface ExtraUsage {
  is_enabled: boolean;
  monthly_limit_cents?: number;
  used_credits_cents?: number;
  utilization?: number;
  resets_at: string | null;
}

export interface UsageSnapshot {
  five_hour: Utilization | null;
  seven_day: Utilization | null;
  seven_day_sonnet: Utilization | null;
  seven_day_opus: Utilization | null;
  extra_usage: ExtraUsage | null;
  fetched_at: string;
}

export interface BurnRateProjection {
  /** Slope of five_hour.utilization in percentage points per minute. */
  utilization_per_min: number;
  /** Projected utilization at five_hour.resets_at if the current pace continues. */
  projected_at_reset: number;
}

export interface CachedUsage {
  snapshot: UsageSnapshot;
  account_id: string;
  account_email: string;
  last_error: string | null;
  burn_rate: BurnRateProjection | null;
}

export interface DailyBucket {
  date: string;
  input_tokens: number;
  output_tokens: number;
  cost_usd: number;
}

export interface ModelStats {
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_creation_tokens: number;
  cost_usd: number;
}

export interface ProjectStats {
  project: string;
  session_count: number;
  total_cost_usd: number;
}

export interface CacheStats {
  total_cache_read_tokens: number;
  total_cache_creation_tokens: number;
  estimated_savings_usd: number;
  hit_ratio: number;
}

export interface SessionEvent {
  ts: string;
  project: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_creation_5m_tokens: number;
  cache_creation_1h_tokens: number;
  cost_usd: number;
  source_file: string;
  source_line: number;
}

export interface Settings {
  polling_interval_secs: number;
  thresholds: number[];
  theme: string;
  launch_at_login: boolean;
  crash_reports: boolean;
}

/* Frontend-only types used by UI components */

export interface HeatmapCell {
  date: string;
  value: number;
  level: 0 | 1 | 2 | 3 | 4;
}

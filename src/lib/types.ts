import type { Model } from './store';

/* ─── Session history ─── */

export interface SessionEvent {
  ts: string;
  project: string;
  model: Model;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  cost_usd: number;
}

/* ─── Model breakdown ─── */

export interface ModelEntry {
  model: Model;
  total_tokens: number;
  cost_usd: number;
  sessions: number;
}

export interface ModelBreakdown {
  models: ModelEntry[];
  total_cost: number;
  total_tokens: number;
}

/* ─── Daily trends ─── */

export interface DailyBucket {
  date: string;
  five_hour_pct: number;
  seven_day_pct: number;
  tokens: number;
  cost_usd: number;
}

/* ─── Project breakdown ─── */

export interface ProjectStats {
  project: string;
  sessions: number;
  total_tokens: number;
  cost_usd: number;
  models: Partial<Record<Model, number>>;
}

/* ─── Heatmap ─── */

export interface HeatmapCell {
  date: string;
  value: number;
  level: 0 | 1 | 2 | 3 | 4;
}

/* ─── Cache stats ─── */

export interface CacheStats {
  cache_read_tokens: number;
  cache_write_tokens: number;
  total_input_tokens: number;
  estimated_savings_usd: number;
  hit_rate_pct: number;
}

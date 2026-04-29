import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CachedUsage } from "./types";

export type AppEvent =
  | { type: "usage_updated"; payload: CachedUsage }
  | { type: "session_ingested"; payload: number }
  | { type: "auth_required" }
  | {
      type: "auth_source_conflict";
      payload: { oauth_email: string; cli_email: string };
    }
  | { type: "stale_data" }
  | { type: "db_reset" }
  | { type: "popover_hidden" }
  | { type: "popover_shown" }
  | { type: "watcher_error"; payload: string };

export function subscribe(
  handler: (e: AppEvent) => void,
): Promise<UnlistenFn[]> {
  return Promise.all([
    listen<CachedUsage>("usage_updated", (e) =>
      handler({ type: "usage_updated", payload: e.payload }),
    ),
    listen<number>("session_ingested", (e) =>
      handler({ type: "session_ingested", payload: e.payload }),
    ),
    listen("auth_required", () => handler({ type: "auth_required" })),
    listen<{ oauth_email: string; cli_email: string }>(
      "auth_source_conflict",
      (e) =>
        handler({ type: "auth_source_conflict", payload: e.payload }),
    ),
    listen("stale_data", () => handler({ type: "stale_data" })),
    listen("db_reset", () => handler({ type: "db_reset" })),
    listen("popover_hidden", () => handler({ type: "popover_hidden" })),
    listen("popover_shown", () => handler({ type: "popover_shown" })),
    listen<string>("watcher_error", (e) =>
      handler({ type: "watcher_error", payload: e.payload }),
    ),
  ]);
}

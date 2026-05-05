-- 0003: Drop placeholder rows from notification_state.
-- v1 wrote "unknown-OAuth" / "unknown-ClaudeCode" as account_id stand-ins;
-- multi-account writes a real accountUuid. Placeholder rows would never
-- match again and would silently suppress the first re-cross. Truncating is
-- cheaper than per-row migration: at most one re-fired notification per
-- already-crossed threshold on the next poll.
DELETE FROM notification_state;

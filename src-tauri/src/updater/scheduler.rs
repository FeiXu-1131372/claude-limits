//! Schedule math for the updater background task. Pure function +
//! tests; the tokio loop that calls it lives in `super::run_scheduler`.

use chrono::{DateTime, Duration, Utc};

#[allow(dead_code)]
pub const CHECK_INTERVAL_HOURS: i64 = 6;

/// How long until the next check should run, given when we last checked
/// (or `None` if never). Returns `Duration::zero()` when overdue.
#[allow(dead_code)]
pub fn delay_until_next_check(
    now: DateTime<Utc>,
    last_checked_at: Option<DateTime<Utc>>,
) -> Duration {
    let interval = Duration::hours(CHECK_INTERVAL_HOURS);
    match last_checked_at {
        None => Duration::zero(), // never checked → check immediately
        Some(prev) => {
            let elapsed = now - prev;
            if elapsed >= interval {
                Duration::zero()
            } else {
                interval - elapsed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use pretty_assertions::assert_eq;

    fn t(h: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 29, 0, 0, 0).unwrap() + Duration::hours(h)
    }

    #[test]
    fn never_checked_means_check_now() {
        assert_eq!(delay_until_next_check(t(0), None), Duration::zero());
    }

    #[test]
    fn just_checked_waits_full_interval() {
        let prev = t(0);
        let now = t(0);
        assert_eq!(delay_until_next_check(now, Some(prev)), Duration::hours(6));
    }

    #[test]
    fn three_hours_ago_waits_three_more() {
        let prev = t(0);
        let now = t(3);
        assert_eq!(delay_until_next_check(now, Some(prev)), Duration::hours(3));
    }

    #[test]
    fn overdue_means_check_now() {
        let prev = t(0);
        let now = t(10);
        assert_eq!(delay_until_next_check(now, Some(prev)), Duration::zero());
    }

    #[test]
    fn exactly_at_interval_means_check_now() {
        let prev = t(0);
        let now = t(6);
        assert_eq!(delay_until_next_check(now, Some(prev)), Duration::zero());
    }
}

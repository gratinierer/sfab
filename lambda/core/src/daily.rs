//! Time in UTC days, and the per-instance list of agents already marked active today:
//! `.stories/2-identity-api/spec.md` "Storage", "The last active day".

use std::collections::HashSet;
use std::sync::Mutex;

/// Seconds in a day.
pub const DAY: i64 = 86_400;

/// The UTC day number of a Unix time.
pub fn utc_day(now: i64) -> i64 {
    now.div_euclid(DAY)
}

/// When the daily world job runs, in seconds after 00:00 UTC: `cdk/world_job.py`'s `SCHEDULE`.
pub const DAILY_RUN_OFFSET: i64 = 5 * 60;

/// The daily world job's next run after `now`, for a `retry_after` that waits for it.
pub fn next_daily_run(now: i64) -> i64 {
    let today = utc_day(now) * DAY + DAILY_RUN_OFFSET;
    if now < today { today } else { today + DAY }
}

/// A Unix time as RFC 3339 in UTC, to the second, such as `2026-11-05T00:00:00Z`.
pub fn rfc3339(now: i64) -> String {
    let (days, secs) = (now.div_euclid(DAY), now.rem_euclid(DAY));
    // Days to a civil date: Howard Hinnant's `civil_from_days`.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// Whether `text` has the form [`rfc3339`] writes, `YYYY-MM-DDTHH:MM:SSZ`, so it compares with
/// those times as text.
pub fn is_rfc3339_second(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 20
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            10 => *c == b'T',
            13 | 16 => *c == b':',
            19 => *c == b'Z',
            _ => c.is_ascii_digit(),
        })
}

/// IDs already marked today, capped, and cleared at midnight UTC or when full. Clearing early
/// only costs an extra write.
#[derive(Debug)]
pub struct MarkedToday {
    cap: usize,
    state: Mutex<(i64, HashSet<String>)>,
}

impl MarkedToday {
    /// The spec's cap: about 50 bytes an ID, so about 0.5 MB.
    pub const CAP: usize = 10_000;

    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            state: Mutex::new((i64::MIN, HashSet::new())),
        }
    }

    /// True the first time an ID is seen on a UTC day, and remembers it.
    pub fn first_today(&self, id: &str, now: i64) -> bool {
        let mut state = self.state.lock().expect("the list's lock");
        let (day, ids) = &mut *state;
        if *day != utc_day(now) || ids.len() >= self.cap {
            *day = utc_day(now);
            ids.clear();
        }
        ids.insert(id.to_owned())
    }
}

impl Default for MarkedToday {
    fn default() -> Self {
        Self::new(Self::CAP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_utc_times() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_793_836_800), "2026-11-05T00:00:00Z");
        assert_eq!(rfc3339(951_782_400 + 3_661), "2000-02-29T01:01:01Z");
        assert_eq!(rfc3339(4_102_444_799), "2099-12-31T23:59:59Z");
    }

    #[test]
    fn recognises_the_form_it_writes() {
        assert!(is_rfc3339_second(&rfc3339(1_793_836_800)));
        assert!(!is_rfc3339_second("2026-11-05T00:00:00+00:00"));
        assert!(!is_rfc3339_second("2026-11-05 00:00:00Z"));
    }

    #[test]
    fn the_next_daily_run_is_the_next_00_05_utc() {
        let midnight = 1_793_836_800;
        assert_eq!(rfc3339(next_daily_run(midnight)), "2026-11-05T00:05:00Z");
        assert_eq!(
            rfc3339(next_daily_run(midnight + 299)),
            "2026-11-05T00:05:00Z"
        );
        assert_eq!(
            rfc3339(next_daily_run(midnight + 300)),
            "2026-11-06T00:05:00Z"
        );
        assert_eq!(
            rfc3339(next_daily_run(midnight + 43_200)),
            "2026-11-06T00:05:00Z"
        );
    }

    #[test]
    fn marks_once_a_day() {
        let marked = MarkedToday::default();
        let noon = 1_793_836_800 + 43_200;
        assert!(marked.first_today("a", noon));
        assert!(!marked.first_today("a", noon + 60));
        assert!(marked.first_today("a", noon + 43_200));
    }

    #[test]
    fn clears_when_full() {
        let marked = MarkedToday::new(2);
        assert!(marked.first_today("a", 0));
        assert!(marked.first_today("b", 0));
        assert!(marked.first_today("c", 0));
        assert!(marked.first_today("a", 0));
    }
}

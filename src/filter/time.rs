use anyhow::Result;
use jiff::{Span, Timestamp, Zoned, civil::DateTime, tz::TimeZone};

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Filter based on time ranges.
#[derive(Debug, PartialEq, Eq)]
pub enum TimeFilter {
    Before(SystemTime),
    After(SystemTime),
}

#[cfg(not(test))]
fn now() -> Zoned {
    Zoned::now()
}

#[cfg(test)]
thread_local! {
    static TESTTIME: std::cell::RefCell<Option<Zoned>> = None.into();
}

/// This allows us to set a specific time when running tests
#[cfg(test)]
fn now() -> Zoned {
    TESTTIME.with_borrow(|reftime| reftime.as_ref().cloned().unwrap_or_else(Zoned::now))
}

impl TimeFilter {
    /// Parses a duration/timestamp/date/`@`-prefixed unix-timestamp string.
    ///
    /// On failure, returns the underlying parse error from the `DateTime`
    /// parser (the calendar-date format, and the most common source of
    /// confusing failures, e.g. `2025-11-31` which is not a valid calendar
    /// date) instead of silently discarding it, so callers can tell the
    /// user *why* their input was rejected.
    fn from_str(s: &str) -> Result<SystemTime> {
        if let Ok(span) = s.parse::<Span>() {
            let datetime = now().checked_sub(span)?;
            return Ok(datetime.into());
        }
        if let Ok(timestamp) = s.parse::<Timestamp>() {
            return Ok(timestamp.into());
        }
        match s.parse::<DateTime>() {
            Ok(datetime) => Ok(TimeZone::system()
                .to_ambiguous_zoned(datetime)
                .later()?
                .into()),
            Err(datetime_err) => {
                if let Some(timestamp_secs) = s.strip_prefix('@')
                    && let Ok(timestamp_secs) = timestamp_secs.parse()
                {
                    return Ok(UNIX_EPOCH + Duration::from_secs(timestamp_secs));
                }
                // None of the supported formats matched. The `DateTime`
                // parser gives the most useful reason for calendar-date-
                // shaped input (the common case for this kind of mistake),
                // so surface that instead of a generic message.
                Err(datetime_err.into())
            }
        }
    }

    pub fn before(s: &str) -> Result<TimeFilter> {
        TimeFilter::from_str(s).map(TimeFilter::Before)
    }

    pub fn after(s: &str) -> Result<TimeFilter> {
        TimeFilter::from_str(s).map(TimeFilter::After)
    }

    pub fn applies_to(&self, t: &SystemTime) -> bool {
        match self {
            TimeFilter::Before(limit) => t < limit,
            TimeFilter::After(limit) => t > limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    struct TestTime(SystemTime);

    impl TestTime {
        fn new(time: Zoned) -> Self {
            TESTTIME.with_borrow_mut(|t| *t = Some(time.clone()));
            TestTime(time.into())
        }

        fn set(&mut self, time: Zoned) {
            TESTTIME.with_borrow_mut(|t| *t = Some(time.clone()));
            self.0 = time.into();
        }

        fn timestamp(&self) -> SystemTime {
            self.0
        }
    }

    impl Drop for TestTime {
        fn drop(&mut self) {
            // Stop using manually set times
            TESTTIME.with_borrow_mut(|t| *t = None);
        }
    }

    #[test]
    fn is_time_filter_applicable() {
        let local_tz = TimeZone::system();
        let mut test_time = TestTime::new(
            local_tz
                .to_ambiguous_zoned("2010-10-10 10:10:10".parse::<DateTime>().unwrap())
                .later()
                .unwrap(),
        );
        let mut ref_time = test_time.timestamp();

        assert!(TimeFilter::after("1min").unwrap().applies_to(&ref_time));
        assert!(!TimeFilter::before("1min").unwrap().applies_to(&ref_time));

        let t1m_ago = ref_time - Duration::from_secs(60);
        assert!(!TimeFilter::after("30sec").unwrap().applies_to(&t1m_ago));
        assert!(TimeFilter::after("2min").unwrap().applies_to(&t1m_ago));

        assert!(TimeFilter::before("30sec").unwrap().applies_to(&t1m_ago));
        assert!(!TimeFilter::before("2min").unwrap().applies_to(&t1m_ago));

        let t10s_before = "2010-10-10 10:10:00";
        assert!(
            !TimeFilter::before(t10s_before)
                .unwrap()
                .applies_to(&ref_time)
        );
        assert!(
            TimeFilter::before(t10s_before)
                .unwrap()
                .applies_to(&t1m_ago)
        );

        assert!(
            TimeFilter::after(t10s_before)
                .unwrap()
                .applies_to(&ref_time)
        );
        assert!(!TimeFilter::after(t10s_before).unwrap().applies_to(&t1m_ago));

        let same_day = "2010-10-10";
        assert!(!TimeFilter::before(same_day).unwrap().applies_to(&ref_time));
        assert!(!TimeFilter::before(same_day).unwrap().applies_to(&t1m_ago));

        assert!(TimeFilter::after(same_day).unwrap().applies_to(&ref_time));
        assert!(TimeFilter::after(same_day).unwrap().applies_to(&t1m_ago));

        test_time.set(
            "2010-10-10T10:10:10+00:00"
                .parse::<Timestamp>()
                .unwrap()
                .to_zoned(local_tz.clone()),
        );
        ref_time = test_time.timestamp();
        let t1m_ago = ref_time - Duration::from_secs(60);
        let t10s_before = "2010-10-10T10:10:00+00:00";
        assert!(
            !TimeFilter::before(t10s_before)
                .unwrap()
                .applies_to(&ref_time)
        );
        assert!(
            TimeFilter::before(t10s_before)
                .unwrap()
                .applies_to(&t1m_ago)
        );

        assert!(
            TimeFilter::after(t10s_before)
                .unwrap()
                .applies_to(&ref_time)
        );
        assert!(!TimeFilter::after(t10s_before).unwrap().applies_to(&t1m_ago));

        let ref_timestamp = 1707723412u64; // Mon Feb 12 07:36:52 UTC 2024
        test_time.set(
            "2024-02-12T07:36:52+00:00"
                .parse::<Timestamp>()
                .unwrap()
                .to_zoned(local_tz),
        );
        ref_time = test_time.timestamp();
        let t1m_ago = ref_time - Duration::from_secs(60);
        let t1s_later = ref_time + Duration::from_secs(1);
        // Timestamp only supported via '@' prefix
        assert!(TimeFilter::before(&ref_timestamp.to_string()).is_err());
        assert!(
            TimeFilter::before(&format!("@{ref_timestamp}"))
                .unwrap()
                .applies_to(&t1m_ago)
        );
        assert!(
            !TimeFilter::before(&format!("@{ref_timestamp}"))
                .unwrap()
                .applies_to(&t1s_later)
        );
        assert!(
            !TimeFilter::after(&format!("@{ref_timestamp}"))
                .unwrap()
                .applies_to(&t1m_ago)
        );
        assert!(
            TimeFilter::after(&format!("@{ref_timestamp}"))
                .unwrap()
                .applies_to(&t1s_later)
        );
    }

    /// Length of the longest contiguous substring shared by `a` and `b`.
    ///
    /// Used to check that two error messages share substantial content
    /// without hardcoding what that content actually says.
    fn longest_common_substring_len(a: &str, b: &str) -> usize {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        let mut prev = vec![0usize; b.len() + 1];
        let mut best = 0;
        for i in 1..=a.len() {
            let mut cur = vec![0usize; b.len() + 1];
            for j in 1..=b.len() {
                if a[i - 1] == b[j - 1] {
                    cur[j] = prev[j - 1] + 1;
                    best = best.max(cur[j]);
                }
            }
            prev = cur;
        }
        best
    }

    #[test]
    fn invalid_calendar_date_error_includes_inner_reason() {
        // The error must surface *why* parsing failed, not merely echo the
        // raw input back under a generic wrapper. We deliberately don't
        // assert on jiff's exact wording (e.g. that it mentions "day"),
        // since that would break on an unrelated jiff version bump. See
        // https://github.com/sharkdp/fd/issues/2053
        //
        // A naive check that two different invalid inputs produce two
        // different messages is gameable: a regression that dropped the
        // real parse reason but still echoed the input (e.g.
        // `format!("could not parse '{s}' as a date")`) would still make
        // the two messages differ, purely because the inputs differ, while
        // never actually surfacing the reason.
        //
        // Instead, compare error text for inputs that are invalid for the
        // *same* underlying reason against error text for an input that's
        // invalid for a *different* reason:
        // - "2025-11-31" and "2019-06-31" both fail because day 31 doesn't
        //   exist in a 30-day month (November / June), despite the raw
        //   strings barely overlapping (different year and month).
        // - "2025-13-01" fails for an unrelated reason (month 13 doesn't
        //   exist).
        // A message that actually carries the reason will make the first
        // pair share a large chunk of text that the third message doesn't
        // share. A message that's just the input echoed into a fixed
        // template would make all three overlap by roughly the same
        // (small) amount, since the only shared content would be the fixed
        // wrapper text.
        for filter in [TimeFilter::before, TimeFilter::after] {
            let same_reason_a = filter("2025-11-31").unwrap_err().to_string();
            let same_reason_b = filter("2019-06-31").unwrap_err().to_string();
            let different_reason = filter("2025-13-01").unwrap_err().to_string();

            assert!(!same_reason_a.is_empty());
            assert_ne!(
                same_reason_a, different_reason,
                "distinct invalid inputs should surface distinct underlying reasons, not a shared generic message"
            );

            let same_reason_overlap = longest_common_substring_len(&same_reason_a, &same_reason_b);
            let different_reason_overlap =
                longest_common_substring_len(&same_reason_a, &different_reason);
            assert!(
                same_reason_overlap >= 20,
                "two dates invalid for the same reason should share substantial error text \
                 (got only {same_reason_overlap} shared characters between {same_reason_a:?} \
                 and {same_reason_b:?})"
            );
            assert!(
                same_reason_overlap > different_reason_overlap + 10,
                "shared text between same-reason errors ({same_reason_overlap} chars) should \
                 clearly exceed shared text between different-reason errors \
                 ({different_reason_overlap} chars): {same_reason_a:?} vs {same_reason_b:?} vs \
                 {different_reason:?}; a smaller gap suggests the message may just be echoing \
                 the input rather than surfacing the actual reason"
            );
        }
    }
}

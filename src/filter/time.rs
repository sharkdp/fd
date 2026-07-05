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

    #[test]
    fn invalid_calendar_date_error_includes_inner_reason() {
        // November only has 30 days, so this is not a valid calendar date.
        // The error should surface the actual underlying parse failure
        // instead of a generic rejection. We deliberately don't assert on
        // jiff's exact wording (e.g. that it mentions "day"), since that
        // would break on an unrelated jiff version bump. Instead, check
        // that the message is non-empty and differs from the message for a
        // differently-malformed input, which shows the inner reason is
        // actually being propagated. See
        // https://github.com/sharkdp/fd/issues/2053
        let day_err = TimeFilter::before("2025-11-31").unwrap_err().to_string();
        let garbage_err = TimeFilter::before("not-a-real-date")
            .unwrap_err()
            .to_string();
        assert!(!day_err.is_empty());
        assert_ne!(
            day_err, garbage_err,
            "distinct invalid inputs should surface distinct underlying reasons, not a shared generic message"
        );

        let day_err = TimeFilter::after("2025-11-31").unwrap_err().to_string();
        let garbage_err = TimeFilter::after("not-a-real-date")
            .unwrap_err()
            .to_string();
        assert!(!day_err.is_empty());
        assert_ne!(
            day_err, garbage_err,
            "distinct invalid inputs should surface distinct underlying reasons, not a shared generic message"
        );
    }
}

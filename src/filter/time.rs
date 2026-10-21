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
    fn from_str(s: &str) -> Result<SystemTime, String> {
        if let Ok(span) = s.parse::<Span>() {
            now()
                .checked_sub(span)
                .map(Into::into)
                .map_err(|e| format!("duration '{s}' is out of range: {e}"))
        } else if let Some(secs) = s.strip_prefix('@') {
            secs.parse::<u64>()
                .ok()
                .and_then(|secs| UNIX_EPOCH.checked_add(Duration::from_secs(secs)))
                .ok_or_else(|| {
                    format!("'{s}' is not a valid unix timestamp: expected '@' followed by a number of seconds since the epoch")
                })
        } else if let Ok(timestamp) = s.parse::<Timestamp>() {
            Ok(timestamp.into())
        } else {
            match s.parse::<DateTime>() {
                Ok(datetime) => TimeZone::system()
                    .to_ambiguous_zoned(datetime)
                    .later()
                    .map(Into::into)
                    .map_err(|e| format!("date '{s}' does not exist in the local time zone: {e}")),
                Err(e) => Err(format!("'{s}' is not a valid date or duration: {e}")),
            }
        }
    }

    pub fn before(s: &str) -> Result<TimeFilter, String> {
        TimeFilter::from_str(s).map(TimeFilter::Before)
    }

    pub fn after(s: &str) -> Result<TimeFilter, String> {
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
    fn out_of_range_unix_timestamp_is_rejected() {
        // A '@' timestamp large enough to overflow SystemTime must return
        // an error rather than panicking.
        assert!(TimeFilter::before(&format!("@{}", u64::MAX)).is_err());
        assert!(TimeFilter::after(&format!("@{}", u64::MAX)).is_err());
    }

    #[test]
    fn error_messages_explain_why_parsing_failed() {
        // A well-formatted date that does not exist in the calendar should say so,
        // instead of only claiming the input is invalid.
        let err = TimeFilter::before("2025-11-31").unwrap_err();
        assert!(
            err.contains("2025-11-31") && err.contains("day"),
            "unexpected error message: {err}"
        );

        let err = TimeFilter::after("not-a-date").unwrap_err();
        assert!(
            err.contains("not-a-date")
                && err.len() > "'not-a-date' is not a valid date or duration".len(),
            "unexpected error message: {err}"
        );

        let err = TimeFilter::before("@nope").unwrap_err();
        assert!(
            err.contains("unix timestamp"),
            "unexpected error message: {err}"
        );
    }
}

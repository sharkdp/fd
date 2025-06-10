use jiff::{civil::DateTime, tz::TimeZone, Span, Timestamp, Zoned};

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
    fn from_str(s: &str) -> Option<SystemTime> {
        if let Ok(span) = s.parse::<Span>() {
            let datetime = now().checked_sub(span).ok()?;
            Some(datetime.into())
        } else if let Ok(timestamp) = s.parse::<Timestamp>() {
            Some(timestamp.into())
        } else if let Ok(datetime) = s.parse::<DateTime>() {
            Some(
                TimeZone::system()
                    .to_ambiguous_zoned(datetime)
                    .later()
                    .ok()?
                    .into(),
            )
        } else {
            let timestamp_secs: u64 = s.strip_prefix('@')?.parse().ok()?;
            Some(UNIX_EPOCH + Duration::from_secs(timestamp_secs))
        }
    }

    pub fn before(s: &str) -> Option<Self> {
        Self::from_str(s).map(TimeFilter::Before)
    }

    pub fn after(s: &str) -> Option<Self> {
        Self::from_str(s).map(TimeFilter::After)
    }

    pub fn applies_to(&self, t: &SystemTime) -> bool {
        match self {
            Self::Before(limit) => t < limit,
            Self::After(limit) => t > limit,
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
        assert!(!TimeFilter::before(t10s_before)
            .unwrap()
            .applies_to(&ref_time));
        assert!(TimeFilter::before(t10s_before)
            .unwrap()
            .applies_to(&t1m_ago));

        assert!(TimeFilter::after(t10s_before)
            .unwrap()
            .applies_to(&ref_time));
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
        assert!(!TimeFilter::before(t10s_before)
            .unwrap()
            .applies_to(&ref_time));
        assert!(TimeFilter::before(t10s_before)
            .unwrap()
            .applies_to(&t1m_ago));

        assert!(TimeFilter::after(t10s_before)
            .unwrap()
            .applies_to(&ref_time));
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
        assert!(TimeFilter::before(&ref_timestamp.to_string()).is_none());
        assert!(TimeFilter::before(&format!("@{ref_timestamp}"))
            .unwrap()
            .applies_to(&t1m_ago));
        assert!(!TimeFilter::before(&format!("@{ref_timestamp}"))
            .unwrap()
            .applies_to(&t1s_later));
        assert!(!TimeFilter::after(&format!("@{ref_timestamp}"))
            .unwrap()
            .applies_to(&t1m_ago));
        assert!(TimeFilter::after(&format!("@{ref_timestamp}"))
            .unwrap()
            .applies_to(&t1s_later));
    }
}

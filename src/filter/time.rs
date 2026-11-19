use anyhow::anyhow;
use jiff::{Span, Timestamp, ZonedDateTime, civil::DateTime, tz::TimeZone};

use std::time::SystemTime;

/// Filter based on time ranges.
#[derive(Debug, PartialEq, Eq)]
pub enum TimeFilter {
    Before(SystemTime),
    After(SystemTime),
}

#[cfg(not(test))]
fn now() -> ZonedDateTime {
    ZonedDateTime::now()
}

#[cfg(test)]
thread_local! {
    static TESTTIME: std::cell::RefCell<Option<ZonedDateTime>> = None.into();
}

/// This allows us to set a specific time when running tests
#[cfg(test)]
fn now() -> ZonedDateTime {
    TESTTIME.with_borrow(|reftime| reftime.as_ref().cloned().unwrap_or_else(ZonedDateTime::now))
}

fn parse_timestamp(s: &str) -> Result<Timestamp, jiff::Error> {
    if let Ok(span) = s.parse::<Span>() {
        let datetime = now().checked_sub(span)?;
        Ok(datetime.into())
    } else if let Ok(timestamp) = s.parse::<Timestamp>() {
        Ok(timestamp)
    } else if let Ok(datetime) = s.parse::<DateTime>() {
        // Would it be better to use "compatible" instead of later?
        // later is what we used before
        TimeZone::system().to_ambiguous_timestamp(datetime).later()
    } else {
        // Is it worth parsing the int ourselves?
        Timestamp::strptime("@%s", s)
    }
}

impl TimeFilter {
    fn from_str(s: &str) -> anyhow::Result<SystemTime> {
        // TODO better error messages
        parse_timestamp(s)
            .and_then(|t| t.try_into())
            .map_err(|_| anyhow!("'{s} is not a valid date or duration. See 'fd --help'."))
    }

    pub fn before(s: &str) -> anyhow::Result<Self> {
        Self::from_str(s).map(TimeFilter::Before)
    }

    pub fn after(s: &str) -> anyhow::Result<Self> {
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
        fn new(time: ZonedDateTime) -> Self {
            TESTTIME.with_borrow_mut(|t| *t = Some(time.clone()));
            TestTime(time.try_into().unwrap())
        }

        fn set(&mut self, time: ZonedDateTime) {
            TESTTIME.with_borrow_mut(|t| *t = Some(time.clone()));
            self.0 = time.try_into().unwrap();
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
                .to_ambiguous_zoned_datetime("2010-10-10 10:10:10".parse::<DateTime>().unwrap())
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
                .to_zoned_datetime(local_tz.clone()),
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
                .to_zoned_datetime(local_tz),
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
        // None rather than panicking.
        assert!(TimeFilter::before(&format!("@{}", u64::MAX)).is_err());
        assert!(TimeFilter::after(&format!("@{}", u64::MAX)).is_err());
    }
}

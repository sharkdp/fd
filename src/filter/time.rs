use chrono::{offset::TimeZone, DateTime, Local, NaiveDate};

use std::time::SystemTime;

/// Filter based on time ranges.
#[derive(Debug, PartialEq)]
pub enum TimeFilter {
    Before(SystemTime),
    After(SystemTime),
}

impl TimeFilter {
    fn from_str(ref_time: &SystemTime, s: &str) -> Option<SystemTime> {
        humantime::parse_duration(s)
            .map(|duration| *ref_time - duration)
            .ok()
            .or_else(|| {
                DateTime::parse_from_rfc3339(s)
                    .map(|dt| dt.into())
                    .ok()
                    .or_else(|| {
                        NaiveDate::parse_from_str(s, "%F")
                            .map(|nd| nd.and_hms(0, 0, 0))
                            .ok()
                            .and_then(|ndt| Local.from_local_datetime(&ndt).single())
                    })
                    .or_else(|| Local.datetime_from_str(s, "%F %T").ok())
                    .map(|dt| dt.into())
            })
    }

    pub fn before(ref_time: &SystemTime, s: &str) -> Option<TimeFilter> {
        TimeFilter::from_str(ref_time, s).map(TimeFilter::Before)
    }

    pub fn after(ref_time: &SystemTime, s: &str) -> Option<TimeFilter> {
        TimeFilter::from_str(ref_time, s).map(TimeFilter::After)
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

    #[test]
    fn is_time_filter_applicable() {
        let ref_time = Local
            .datetime_from_str("2010-10-10 10:10:10", "%F %T")
            .unwrap()
            .into();

        assert!(TimeFilter::after(&ref_time, "1min")
            .unwrap()
            .applies_to(&ref_time));
        assert!(!TimeFilter::before(&ref_time, "1min")
            .unwrap()
            .applies_to(&ref_time));

        let t1m_ago = ref_time - Duration::from_secs(60);
        assert!(!TimeFilter::after(&ref_time, "30sec")
            .unwrap()
            .applies_to(&t1m_ago));
        assert!(TimeFilter::after(&ref_time, "2min")
            .unwrap()
            .applies_to(&t1m_ago));

        assert!(TimeFilter::before(&ref_time, "30sec")
            .unwrap()
            .applies_to(&t1m_ago));
        assert!(!TimeFilter::before(&ref_time, "2min")
            .unwrap()
            .applies_to(&t1m_ago));

        let t10s_before = "2010-10-10 10:10:00";
        assert!(!TimeFilter::before(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&ref_time));
        assert!(TimeFilter::before(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&t1m_ago));

        assert!(TimeFilter::after(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&ref_time));
        assert!(!TimeFilter::after(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&t1m_ago));

        let same_day = "2010-10-10";
        assert!(!TimeFilter::before(&ref_time, same_day)
            .unwrap()
            .applies_to(&ref_time));
        assert!(!TimeFilter::before(&ref_time, same_day)
            .unwrap()
            .applies_to(&t1m_ago));

        assert!(TimeFilter::after(&ref_time, same_day)
            .unwrap()
            .applies_to(&ref_time));
        assert!(TimeFilter::after(&ref_time, same_day)
            .unwrap()
            .applies_to(&t1m_ago));

        let ref_time = DateTime::parse_from_rfc3339("2010-10-10T10:10:10+00:00")
            .unwrap()
            .into();
        let t1m_ago = ref_time - Duration::from_secs(60);
        let t10s_before = "2010-10-10T10:10:00+00:00";
        assert!(!TimeFilter::before(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&ref_time));
        assert!(TimeFilter::before(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&t1m_ago));

        assert!(TimeFilter::after(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&ref_time));
        assert!(!TimeFilter::after(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&t1m_ago));
    }
}

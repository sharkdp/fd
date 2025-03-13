use jiff::{civil::DateTime, tz::TimeZone, Span, Timestamp, Zoned};

use std::time::SystemTime;

/// Filter based on time ranges.
#[derive(Debug, PartialEq, Eq)]
pub enum TimeFilter {
    Before(SystemTime),
    After(SystemTime),
}

impl TimeFilter {
    fn from_str(ref_time: &SystemTime, s: &str) -> Option<SystemTime> {
        s.parse::<Span>()
            .and_then(|duration| {
                Zoned::try_from(*ref_time).and_then(|zdt| zdt.checked_sub(duration))
            })
            .ok()
            .or_else(|| {
                let local_tz = TimeZone::system();
                s.parse::<Timestamp>()
                    .map(|ts| ts.to_zoned(TimeZone::UTC))
                    .ok()
                    .or_else(|| {
                        s.parse::<DateTime>()
                            .map(|dt| local_tz.to_ambiguous_zoned(dt))
                            .and_then(|zdt| zdt.later())
                            .ok()
                    })
                    .or_else(|| {
                        let timestamp_secs = s.strip_prefix('@')?.parse().ok()?;
                        Timestamp::from_second(timestamp_secs)
                            .map(|ts| ts.to_zoned(TimeZone::UTC))
                            .ok()
                    })
            })
            .map(SystemTime::from)
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
        let local_tz = TimeZone::system();
        let ref_time = local_tz
            .to_ambiguous_zoned("2010-10-10 10:10:10".parse::<DateTime>().unwrap())
            .later()
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

        let ref_time = "2010-10-10T10:10:10+00:00"
            .parse::<Timestamp>()
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

        let ref_timestamp = 1707723412u64; // Mon Feb 12 07:36:52 UTC 2024
        let ref_time = "2024-02-12T07:36:52+00:00"
            .parse::<Timestamp>()
            .unwrap()
            .into();
        let t1m_ago = ref_time - Duration::from_secs(60);
        let t1s_later = ref_time + Duration::from_secs(1);
        // Timestamp only supported via '@' prefix
        assert!(TimeFilter::before(&ref_time, &ref_timestamp.to_string()).is_none());
        assert!(TimeFilter::before(&ref_time, &format!("@{ref_timestamp}"))
            .unwrap()
            .applies_to(&t1m_ago));
        assert!(!TimeFilter::before(&ref_time, &format!("@{ref_timestamp}"))
            .unwrap()
            .applies_to(&t1s_later));
        assert!(!TimeFilter::after(&ref_time, &format!("@{ref_timestamp}"))
            .unwrap()
            .applies_to(&t1m_ago));
        assert!(TimeFilter::after(&ref_time, &format!("@{ref_timestamp}"))
            .unwrap()
            .applies_to(&t1s_later));
    }
}

use chrono::{offset::TimeZone, DateTime, Local, NaiveDate};

use std::time::SystemTime;

#[derive(Debug, PartialEq)]
pub enum TimeRange {
    Before(SystemTime),
    After(SystemTime),
}

impl TimeRange {
    pub fn from_str(
        ref_time: &SystemTime,
        s: &str,
        variant: fn(SystemTime) -> TimeRange,
    ) -> Option<Self> {
        let time = humantime::parse_duration(s)
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
            })?;
        Some(variant(time))
    }

    #[cfg(test)]
    fn after(ref_time: &SystemTime, s: &str) -> Option<Self> {
        Self::from_str(ref_time, s, Self::After)
    }

    #[cfg(test)]
    fn before(ref_time: &SystemTime, s: &str) -> Option<Self> {
        Self::from_str(ref_time, s, Self::Before)
    }

    pub fn applies_to(&self, t: &SystemTime) -> bool {
        match self {
            TimeRange::Before(limit) => t < limit,
            TimeRange::After(limit) => t > limit,
        }
    }
}

/// Which file time to filter on.
#[derive(Debug, PartialEq)]
pub enum TimeFilterKind {
    Modified,
    Accessed,
    Created,
}

/// Filter based on time ranges.
#[derive(Debug, PartialEq)]
pub struct TimeFilter {
    kind: TimeFilterKind,
    range: TimeRange,
}

impl TimeFilter {
    pub fn new(
        kind: TimeFilterKind,
        variant: fn(SystemTime) -> TimeRange,
        text: &str,
        ref_time: &SystemTime,
    ) -> Option<Self> {
        Some(Self {
            kind,
            range: TimeRange::from_str(ref_time, text, variant)?,
        })
    }

    pub fn applies_to(&self, m: &std::fs::Metadata) -> bool {
        let res = match self.kind {
            TimeFilterKind::Modified => m.modified(),
            TimeFilterKind::Accessed => m.accessed(),
            TimeFilterKind::Created => m.created(),
        };
        if let Ok(time) = res {
            self.range.applies_to(&time)
        } else {
            false
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

        assert!(TimeRange::after(&ref_time, "1min")
            .unwrap()
            .applies_to(&ref_time));
        assert!(!TimeRange::before(&ref_time, "1min")
            .unwrap()
            .applies_to(&ref_time));

        let t1m_ago = ref_time - Duration::from_secs(60);
        assert!(!TimeRange::after(&ref_time, "30sec")
            .unwrap()
            .applies_to(&t1m_ago));
        assert!(TimeRange::after(&ref_time, "2min")
            .unwrap()
            .applies_to(&t1m_ago));

        assert!(TimeRange::before(&ref_time, "30sec")
            .unwrap()
            .applies_to(&t1m_ago));
        assert!(!TimeRange::before(&ref_time, "2min")
            .unwrap()
            .applies_to(&t1m_ago));

        let t10s_before = "2010-10-10 10:10:00";
        assert!(!TimeRange::before(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&ref_time));
        assert!(TimeRange::before(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&t1m_ago));

        assert!(TimeRange::after(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&ref_time));
        assert!(!TimeRange::after(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&t1m_ago));

        let same_day = "2010-10-10";
        assert!(!TimeRange::before(&ref_time, same_day)
            .unwrap()
            .applies_to(&ref_time));
        assert!(!TimeRange::before(&ref_time, same_day)
            .unwrap()
            .applies_to(&t1m_ago));

        assert!(TimeRange::after(&ref_time, same_day)
            .unwrap()
            .applies_to(&ref_time));
        assert!(TimeRange::after(&ref_time, same_day)
            .unwrap()
            .applies_to(&t1m_ago));

        let ref_time = DateTime::parse_from_rfc3339("2010-10-10T10:10:10+00:00")
            .unwrap()
            .into();
        let t1m_ago = ref_time - Duration::from_secs(60);
        let t10s_before = "2010-10-10T10:10:00+00:00";
        assert!(!TimeRange::before(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&ref_time));
        assert!(TimeRange::before(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&t1m_ago));

        assert!(TimeRange::after(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&ref_time));
        assert!(!TimeRange::after(&ref_time, t10s_before)
            .unwrap()
            .applies_to(&t1m_ago));
    }
}

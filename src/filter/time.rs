use time::format_description::{parse as time_format, well_known::Rfc3339};
use time::{Date, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use std::time::SystemTime;

/// Filter based on time ranges.
#[derive(Debug, PartialEq, Eq)]
pub enum TimeFilter {
    Before(SystemTime),
    After(SystemTime),
}

fn parse_local_time(s: &str) -> time::Result<OffsetDateTime> {
    const DAY_FORMAT: &'static str = "[year]-[month]-[day]";
    const DATETIME_FORMAT: &'static str = "[year]-[month]-[day] [hour repr:24]:[minute]:[second]";
    let primitive = Date::parse(s, &time_format(DAY_FORMAT).unwrap())
        .map(|d| d.midnight())
        .or_else(|_| PrimitiveDateTime::parse(s, &time_format(DATETIME_FORMAT).unwrap()))?;
    let offset = get_offset_for_local_time(primitive)?;
    let local_time = primitive.assume_offset(offset);

    Ok(local_time)
}

#[cfg(not(test))]
fn get_offset_for_local_time(
    time: PrimitiveDateTime,
) -> Result<UtcOffset, time::error::IndeterminateOffset> {
    UtcOffset::local_offset_at(time.assume_utc())
}

/// While running tests, there can be multiple threads, and `UtcOffset::local_offset_at` will fail
/// on unix if there are multiple threads, so during tests, we use a shim that just always returns
/// an offset of -4
#[cfg(test)]
fn get_offset_for_local_time(
    _time: PrimitiveDateTime,
) -> Result<UtcOffset, time::error::ComponentRange> {
    UtcOffset::from_hms(-4, 0, 0)
}

impl TimeFilter {
    fn from_str(ref_time: &SystemTime, s: &str) -> Option<SystemTime> {
        humantime::parse_duration(s)
            .map(|duration| *ref_time - duration)
            .ok()
            .or_else(|| {
                OffsetDateTime::parse(s, &Rfc3339)
                    .or_else(|_| parse_local_time(s))
                    .map(|dt| dt.into())
                    .ok()
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
        let ref_time = parse_local_time("2010-10-10 10:10:10").unwrap().into();

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

        let ref_time = OffsetDateTime::parse("2010-10-10T10:10:10+00:00", &Rfc3339)
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

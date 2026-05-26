use std::sync::OnceLock;

use anyhow::anyhow;
use regex::Regex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinksFilter {
    Max(u64),
    Min(u64),
    Equals(u64),
}

static LINKS_CAPTURES: OnceLock<Regex> = OnceLock::new();

impl LinksFilter {
    pub fn from_string(s: &str) -> anyhow::Result<Self> {
        LinksFilter::parse_opt(s)
            .ok_or_else(|| anyhow!("'{}' is not a valid link count constraint. See 'fd --help'.", s))
    }

    fn parse_opt(s: &str) -> Option<Self> {
        let pattern = LINKS_CAPTURES.get_or_init(|| Regex::new(r"^([+-]?)(\d+)$").unwrap());
        let captures = pattern.captures(s)?;
        let limit_kind = captures.get(1).map_or("", |m| m.as_str());
        let count = captures
            .get(2)
            .and_then(|v| v.as_str().parse::<u64>().ok())?;

        match limit_kind {
            "+" => Some(LinksFilter::Min(count)),
            "-" => Some(LinksFilter::Max(count)),
            "" => Some(LinksFilter::Equals(count)),
            _ => None,
        }
    }

    pub fn is_within(&self, links: u64) -> bool {
        match *self {
            LinksFilter::Max(limit) => links <= limit,
            LinksFilter::Min(limit) => links >= limit,
            LinksFilter::Equals(limit) => links == limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_exact_link_count() {
        assert_eq!(
            LinksFilter::from_string("2").unwrap(),
            LinksFilter::Equals(2)
        );
    }

    #[test]
    fn parse_minimum_link_count() {
        assert_eq!(
            LinksFilter::from_string("+2").unwrap(),
            LinksFilter::Min(2)
        );
    }

    #[test]
    fn parse_maximum_link_count() {
        assert_eq!(
            LinksFilter::from_string("-1").unwrap(),
            LinksFilter::Max(1)
        );
    }

    #[test]
    fn is_within_exact() {
        let filter = LinksFilter::Equals(2);
        assert!(filter.is_within(2));
        assert!(!filter.is_within(1));
    }

    #[test]
    fn is_within_minimum() {
        let filter = LinksFilter::Min(2);
        assert!(filter.is_within(2));
        assert!(filter.is_within(3));
        assert!(!filter.is_within(1));
    }

    #[test]
    fn is_within_maximum() {
        let filter = LinksFilter::Max(1);
        assert!(filter.is_within(1));
        assert!(filter.is_within(0));
        assert!(!filter.is_within(2));
    }
}

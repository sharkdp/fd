use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref PERM_CAPTURES: Regex = { Regex::new(r"[0-7]{3}$").unwrap() };
}

#[derive(Debug, PartialEq)]
pub enum PermFilter {
    Permission(u32),
}

impl PermFilter {
    pub fn from_string(s: &str) -> Option<Self> {
        if PERM_CAPTURES.is_match(s) {
            Some(PermFilter::Permission(u32::from_str_radix(s, 8).unwrap()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn matches_all_access() {
        let f = PermFilter::from_string("777").unwrap();
        assert_eq!(f, PermFilter::Permission(0o777));
    }

    #[test]
    fn matches_owner_rwx_others_r() {
        let f = PermFilter::from_string("744").unwrap();
        assert_eq!(f, PermFilter::Permission(0o744));
    }

    #[test]
    fn matches_r_everybody() {
        let f = PermFilter::from_string("444").unwrap();
        assert_eq!(f, PermFilter::Permission(0o444));
    }

    #[test]
    fn matches_no_perm() {
        let f = PermFilter::from_string("000").unwrap();
        assert_eq!(f, PermFilter::Permission(0o000));
    }

    #[test]
    fn dont_match_no_digits() {
        let f = PermFilter::from_string("abc");
        assert_eq!(f, None);
    }

    #[test]
    fn dont_match_greater_number() {
        let f = PermFilter::from_string("800");
        assert_eq!(f, None);
    }

    #[test]
    fn dont_match_empty() {
        let f = PermFilter::from_string("");
        assert_eq!(f, None);
    }

}

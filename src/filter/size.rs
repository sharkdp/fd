use once_cell::sync::Lazy;
use regex::Regex;

static SIZE_CAPTURES: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^([+-]?)(\d+)(b|[kmgt]i?b?)$").unwrap());

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeFilter {
    Max(u64),
    Min(u64),
    Equals(u64),
}

// SI prefixes (powers of 10)
const KILO: u64 = 1000;
const MEGA: u64 = KILO * 1000;
const GIGA: u64 = MEGA * 1000;
const TERA: u64 = GIGA * 1000;

// Binary prefixes (powers of 2)
const KIBI: u64 = 1024;
const MEBI: u64 = KIBI * 1024;
const GIBI: u64 = MEBI * 1024;
const TEBI: u64 = GIBI * 1024;

impl SizeFilter {
    pub fn from_string(s: &str) -> Option<Self> {
        if !SIZE_CAPTURES.is_match(s) {
            return None;
        }

        let captures = SIZE_CAPTURES.captures(s)?;
        let limit_kind = captures.get(1).map_or("+", |m| m.as_str());
        let quantity = captures
            .get(2)
            .and_then(|v| v.as_str().parse::<u64>().ok())?;

        let multiplier = match &captures.get(3).map_or("b", |m| m.as_str()).to_lowercase()[..] {
            v if v.starts_with("ki") => KIBI,
            v if v.starts_with('k') => KILO,
            v if v.starts_with("mi") => MEBI,
            v if v.starts_with('m') => MEGA,
            v if v.starts_with("gi") => GIBI,
            v if v.starts_with('g') => GIGA,
            v if v.starts_with("ti") => TEBI,
            v if v.starts_with('t') => TERA,
            "b" => 1,
            _ => return None,
        };

        let size = quantity * multiplier;
        match limit_kind {
            "+" => Some(SizeFilter::Min(size)),
            "-" => Some(SizeFilter::Max(size)),
            "" => Some(SizeFilter::Equals(size)),
            _ => None,
        }
    }

    pub fn is_within(&self, size: u64) -> bool {
        match *self {
            SizeFilter::Max(limit) => size <= limit,
            SizeFilter::Min(limit) => size >= limit,
            SizeFilter::Equals(limit) => size == limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! gen_size_filter_parse_test {
        ($($name: ident: $val: expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (txt, expected) = $val;
                    let actual = SizeFilter::from_string(txt).unwrap();
                    assert_eq!(actual, expected);
                }
            )*
        };
    }

    // Parsing and size conversion tests data. Ensure that each type gets properly interpreted.
    // Call with higher base values to ensure expected multiplication (only need a couple)
    gen_size_filter_parse_test! {
        byte_plus:                ("+1b",     SizeFilter::Min(1)),
        byte_plus_multiplier:     ("+10b",    SizeFilter::Min(10)),
        byte_minus:               ("-1b",     SizeFilter::Max(1)),
        kilo_plus:                ("+1k",     SizeFilter::Min(1000)),
        kilo_plus_suffix:         ("+1kb",    SizeFilter::Min(1000)),
        kilo_minus:               ("-1k",     SizeFilter::Max(1000)),
        kilo_minus_multiplier:    ("-100k",   SizeFilter::Max(100_000)),
        kilo_minus_suffix:        ("-1kb",    SizeFilter::Max(1000)),
        kilo_plus_upper:          ("+1K",     SizeFilter::Min(1000)),
        kilo_plus_suffix_upper:   ("+1KB",    SizeFilter::Min(1000)),
        kilo_minus_upper:         ("-1K",     SizeFilter::Max(1000)),
        kilo_minus_suffix_upper:  ("-1Kb",    SizeFilter::Max(1000)),
        kibi_plus:                ("+1ki",    SizeFilter::Min(1024)),
        kibi_plus_multiplier:     ("+10ki",   SizeFilter::Min(10_240)),
        kibi_plus_suffix:         ("+1kib",   SizeFilter::Min(1024)),
        kibi_minus:               ("-1ki",    SizeFilter::Max(1024)),
        kibi_minus_multiplier:    ("-100ki",  SizeFilter::Max(102_400)),
        kibi_minus_suffix:        ("-1kib",   SizeFilter::Max(1024)),
        kibi_plus_upper:          ("+1KI",    SizeFilter::Min(1024)),
        kibi_plus_suffix_upper:   ("+1KiB",   SizeFilter::Min(1024)),
        kibi_minus_upper:         ("-1Ki",    SizeFilter::Max(1024)),
        kibi_minus_suffix_upper:  ("-1KIB",   SizeFilter::Max(1024)),
        mega_plus:                ("+1m",     SizeFilter::Min(1_000_000)),
        mega_plus_suffix:         ("+1mb",    SizeFilter::Min(1_000_000)),
        mega_minus:               ("-1m",     SizeFilter::Max(1_000_000)),
        mega_minus_suffix:        ("-1mb",    SizeFilter::Max(1_000_000)),
        mega_plus_upper:          ("+1M",     SizeFilter::Min(1_000_000)),
        mega_plus_suffix_upper:   ("+1MB",    SizeFilter::Min(1_000_000)),
        mega_minus_upper:         ("-1M",     SizeFilter::Max(1_000_000)),
        mega_minus_suffix_upper:  ("-1Mb",    SizeFilter::Max(1_000_000)),
        mebi_plus:                ("+1mi",    SizeFilter::Min(1_048_576)),
        mebi_plus_suffix:         ("+1mib",   SizeFilter::Min(1_048_576)),
        mebi_minus:               ("-1mi",    SizeFilter::Max(1_048_576)),
        mebi_minus_suffix:        ("-1mib",   SizeFilter::Max(1_048_576)),
        mebi_plus_upper:          ("+1MI",    SizeFilter::Min(1_048_576)),
        mebi_plus_suffix_upper:   ("+1MiB",   SizeFilter::Min(1_048_576)),
        mebi_minus_upper:         ("-1Mi",    SizeFilter::Max(1_048_576)),
        mebi_minus_suffix_upper:  ("-1MIB",   SizeFilter::Max(1_048_576)),
        giga_plus:                ("+1g",     SizeFilter::Min(1_000_000_000)),
        giga_plus_suffix:         ("+1gb",    SizeFilter::Min(1_000_000_000)),
        giga_minus:               ("-1g",     SizeFilter::Max(1_000_000_000)),
        giga_minus_suffix:        ("-1gb",    SizeFilter::Max(1_000_000_000)),
        giga_plus_upper:          ("+1G",     SizeFilter::Min(1_000_000_000)),
        giga_plus_suffix_upper:   ("+1GB",    SizeFilter::Min(1_000_000_000)),
        giga_minus_upper:         ("-1G",     SizeFilter::Max(1_000_000_000)),
        giga_minus_suffix_upper:  ("-1Gb",    SizeFilter::Max(1_000_000_000)),
        gibi_plus:                ("+1gi",    SizeFilter::Min(1_073_741_824)),
        gibi_plus_suffix:         ("+1gib",   SizeFilter::Min(1_073_741_824)),
        gibi_minus:               ("-1gi",    SizeFilter::Max(1_073_741_824)),
        gibi_minus_suffix:        ("-1gib",   SizeFilter::Max(1_073_741_824)),
        gibi_plus_upper:          ("+1GI",    SizeFilter::Min(1_073_741_824)),
        gibi_plus_suffix_upper:   ("+1GiB",   SizeFilter::Min(1_073_741_824)),
        gibi_minus_upper:         ("-1Gi",    SizeFilter::Max(1_073_741_824)),
        gibi_minus_suffix_upper:  ("-1GIB",   SizeFilter::Max(1_073_741_824)),
        tera_plus:                ("+1t",     SizeFilter::Min(1_000_000_000_000)),
        tera_plus_suffix:         ("+1tb",    SizeFilter::Min(1_000_000_000_000)),
        tera_minus:               ("-1t",     SizeFilter::Max(1_000_000_000_000)),
        tera_minus_suffix:        ("-1tb",    SizeFilter::Max(1_000_000_000_000)),
        tera_plus_upper:          ("+1T",     SizeFilter::Min(1_000_000_000_000)),
        tera_plus_suffix_upper:   ("+1TB",    SizeFilter::Min(1_000_000_000_000)),
        tera_minus_upper:         ("-1T",     SizeFilter::Max(1_000_000_000_000)),
        tera_minus_suffix_upper:  ("-1Tb",    SizeFilter::Max(1_000_000_000_000)),
        tebi_plus:                ("+1ti",    SizeFilter::Min(1_099_511_627_776)),
        tebi_plus_suffix:         ("+1tib",   SizeFilter::Min(1_099_511_627_776)),
        tebi_minus:               ("-1ti",    SizeFilter::Max(1_099_511_627_776)),
        tebi_minus_suffix:        ("-1tib",   SizeFilter::Max(1_099_511_627_776)),
        tebi_plus_upper:          ("+1TI",    SizeFilter::Min(1_099_511_627_776)),
        tebi_plus_suffix_upper:   ("+1TiB",   SizeFilter::Min(1_099_511_627_776)),
        tebi_minus_upper:         ("-1Ti",    SizeFilter::Max(1_099_511_627_776)),
        tebi_minus_suffix_upper:  ("-1TIB",   SizeFilter::Max(1_099_511_627_776)),
    }

    /// Invalid parse testing
    macro_rules! gen_size_filter_failure {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let i = SizeFilter::from_string($value);
                    assert!(i.is_none());
                }
            )*
        };
    }

    // Invalid parse data
    gen_size_filter_failure! {
        ensure_missing_number_returns_none: "+g",
        ensure_missing_unit_returns_none: "+18",
        ensure_bad_format_returns_none_1: "$10M",
        ensure_bad_format_returns_none_2: "badval",
        ensure_bad_format_returns_none_3: "9999",
        ensure_invalid_unit_returns_none_1: "+50a",
        ensure_invalid_unit_returns_none_2: "-10v",
        ensure_invalid_unit_returns_none_3: "+1Mv",
        ensure_bib_format_returns_none: "+1bib",
        ensure_bb_format_returns_none: "+1bb",
    }

    #[test]
    fn is_within_less_than() {
        let f = SizeFilter::from_string("-1k").unwrap();
        assert!(f.is_within(999));
    }

    #[test]
    fn is_within_less_than_equal() {
        let f = SizeFilter::from_string("-1k").unwrap();
        assert!(f.is_within(1000));
    }

    #[test]
    fn is_within_greater_than() {
        let f = SizeFilter::from_string("+1k").unwrap();
        assert!(f.is_within(1001));
    }

    #[test]
    fn is_within_greater_than_equal() {
        let f = SizeFilter::from_string("+1K").unwrap();
        assert!(f.is_within(1000));
    }
}

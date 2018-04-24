// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::time;

use exec::CommandTemplate;
use lscolors::LsColors;
use regex::{Regex, RegexSet};
use regex_syntax::hir::Hir;
use regex_syntax::Parser;

lazy_static! {
    static ref SIZE_CAPTURES: Regex = { Regex::new(r"(?i)^([+-])(\d+)([bkmgt]i?)b?$").unwrap() };
}

/// Whether or not to show
pub struct FileTypes {
    pub files: bool,
    pub directories: bool,
    pub symlinks: bool,
    pub executables_only: bool,
}

impl Default for FileTypes {
    fn default() -> FileTypes {
        FileTypes {
            files: false,
            directories: false,
            symlinks: false,
            executables_only: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SizeLimitType {
    Max,
    Min,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizeFilter {
    size: u64,
    limit_type: SizeLimitType,
}

// SI units for now
const KILO: u64 = 1000;
const MEGA: u64 = KILO * 1000;
const GIGA: u64 = MEGA * 1000;
const TERA: u64 = GIGA * 1000;

const KIBI: u64 = 1024;
const MEBI: u64 = KIBI * 1024;
const GIBI: u64 = MEBI * 1024;
const TEBI: u64 = GIBI * 1024;

impl SizeFilter {
    pub fn from_string<'a>(s: &str) -> Option<Self> {
        if !SIZE_CAPTURES.is_match(s) {
            return None;
        }

        let captures = match SIZE_CAPTURES.captures(s) {
            Some(cap) => cap,
            None => return None,
        };

        let limit = match captures.get(1).map_or("+", |m| m.as_str()) {
            "+" => SizeLimitType::Min,
            _ => SizeLimitType::Max,
        };

        let quantity = match captures.get(2) {
            None => return None,
            Some(v) => match v.as_str().parse::<u64>() {
                Ok(val) => val,
                _ => return None,
            },
        };

        let multiplier = match &captures.get(3).map_or("k", |m| m.as_str()).to_lowercase()[..] {
            "ki" => KIBI,
            "k" => KILO,
            "mi" => MEBI,
            "m" => MEGA,
            "gi" => GIBI,
            "g" => GIGA,
            "ti" => TEBI,
            "t" => TERA,
            _ => 1, // Any we don't understand we'll just say the number of bytes
        };

        Some(SizeFilter {
            size: quantity * multiplier,
            limit_type: limit,
        })
    }

    pub fn is_within(&self, size: u64) -> bool {
        match self.limit_type {
            SizeLimitType::Max => size <= self.size,
            SizeLimitType::Min => size >= self.size,
        }
    }
}

/// Configuration options for *fd*.
pub struct FdOptions {
    /// Whether the search is case-sensitive or case-insensitive.
    pub case_sensitive: bool,

    /// Whether to search within the full file path or just the base name (filename or directory
    /// name).
    pub search_full_path: bool,

    /// Whether to ignore hidden files and directories (or not).
    pub ignore_hidden: bool,

    /// Whether to respect `.fdignore` files or not.
    pub read_fdignore: bool,

    /// Whether to respect VCS ignore files (`.gitignore`, ..) or not.
    pub read_vcsignore: bool,

    /// Whether to follow symlinks or not.
    pub follow_links: bool,

    /// Whether elements of output should be separated by a null character
    pub null_separator: bool,

    /// The maximum search depth, or `None` if no maximum search depth should be set.
    ///
    /// A depth of `1` includes all files under the current directory, a depth of `2` also includes
    /// all files under subdirectories of the current directory, etc.
    pub max_depth: Option<usize>,

    /// The number of threads to use.
    pub threads: usize,

    /// Time to buffer results internally before streaming to the console. This is useful to
    /// provide a sorted output, in case the total execution time is shorter than
    /// `max_buffer_time`.
    pub max_buffer_time: Option<time::Duration>,

    /// `None` if the output should not be colorized. Otherwise, a `LsColors` instance that defines
    /// how to style different filetypes.
    pub ls_colors: Option<LsColors>,

    /// The type of file to search for. If set to `None`, all file types are displayed. If
    /// set to `Some(..)`, only the types that are specified are shown.
    pub file_types: Option<FileTypes>,

    /// The extension to search for. Only entries matching the extension will be included.
    ///
    /// The value (if present) will be a lowercase string without leading dots.
    pub extensions: Option<RegexSet>,

    /// If a value is supplied, each item found will be used to generate and execute commands.
    pub command: Option<CommandTemplate>,

    /// A list of glob patterns that should be excluded from the search.
    pub exclude_patterns: Vec<String>,

    /// A list of custom ignore files.
    pub ignore_files: Vec<PathBuf>,

    /// The given constraints on the size of returned files
    pub size_constraints: Vec<SizeFilter>,
}

/// Print error message to stderr and exit with status `1`.
pub fn error(message: &str) -> ! {
    writeln!(&mut ::std::io::stderr(), "{}", message).expect("Failed writing to stderr");
    process::exit(1);
}

/// Determine if a regex pattern contains a literal uppercase character.
pub fn pattern_has_uppercase_char(pattern: &str) -> bool {
    Parser::new()
        .parse(pattern)
        .map(|hir| hir_has_uppercase_char(&hir))
        .unwrap_or(false)
}

/// Determine if a regex expression contains a literal uppercase character.
fn hir_has_uppercase_char(hir: &Hir) -> bool {
    use regex_syntax::hir::*;

    match *hir.kind() {
        HirKind::Literal(Literal::Unicode(c)) => c.is_uppercase(),
        HirKind::Class(Class::Unicode(ref ranges)) => ranges
            .iter()
            .any(|r| r.start().is_uppercase() || r.end().is_uppercase()),
        HirKind::Group(Group { ref hir, .. }) | HirKind::Repetition(Repetition { ref hir, .. }) => {
            hir_has_uppercase_char(hir)
        }
        HirKind::Concat(ref hirs) | HirKind::Alternation(ref hirs) => {
            hirs.iter().any(hir_has_uppercase_char)
        }
        _ => false,
    }
}

/// Maximum size of the output buffer before flushing results to the console
pub const MAX_BUFFER_LENGTH: usize = 1000;

/// Exit code representing a general error
pub const EXITCODE_ERROR: i32 = 1;

/// Exit code representing that the process was killed by SIGINT
pub const EXITCODE_SIGINT: i32 = 130;

/// Traverse args_os, looking for -exec and replacing it with --exec.
///
/// # Returns
///
/// * The args, with substitution if required
pub fn transform_args_with_exec<I>(original: I) -> Vec<OsString>
where
    I: Iterator<Item = OsString>,
{
    let mut in_exec_opt = false;
    let target = OsString::from("-exec");
    let long_start = OsString::from("--exec");
    let short_start = OsString::from("-x");
    let exec_end = OsString::from(";");

    original.fold(vec![], |mut args, curr| {
        if in_exec_opt {
            if curr == exec_end {
                in_exec_opt = false;
            }
            args.push(curr);
            return args;
        }

        if curr == target || curr == long_start || curr == short_start {
            args.push(if curr == target {
                OsString::from("--exec")
            } else {
                curr
            });
            in_exec_opt = true;
        } else {
            args.push(curr);
        }
        args
    })
}

#[cfg(test)]
fn oss(v: &str) -> OsString {
    OsString::from(v)
}

/// Ensure that -exec gets transformed into --exec
#[test]
fn normal_exec_substitution() {
    let original = vec![oss("fd"), oss("foo"), oss("-exec"), oss("cmd")];
    let expected = vec![oss("fd"), oss("foo"), oss("--exec"), oss("cmd")];

    let actual = transform_args_with_exec(original.into_iter());
    assert_eq!(expected, actual);
}

/// Ensure that --exec is not touched
#[test]
fn passthru_of_original_exec() {
    let original = vec![oss("fd"), oss("foo"), oss("--exec"), oss("cmd")];
    let expected = vec![oss("fd"), oss("foo"), oss("--exec"), oss("cmd")];

    let actual = transform_args_with_exec(original.into_iter());
    assert_eq!(expected, actual);
}

#[test]
fn temp_check_that_exec_context_observed() {
    let original = vec![
        oss("fd"),
        oss("foo"),
        oss("-exec"),
        oss("cmd"),
        oss("-exec"),
        oss("ls"),
        oss(";"),
        oss("-exec"),
        oss("rm"),
        oss(";"),
        oss("--exec"),
        oss("find"),
        oss("-exec"),
        oss("rm"),
        oss(";"),
        oss("-x"),
        oss("foo"),
        oss("-exec"),
        oss("something"),
        oss(";"),
        oss("-exec"),
    ];
    let expected = vec![
        oss("fd"),
        oss("foo"),
        oss("--exec"),
        oss("cmd"),
        oss("-exec"),
        oss("ls"),
        oss(";"),
        oss("--exec"),
        oss("rm"),
        oss(";"),
        oss("--exec"),
        oss("find"),
        oss("-exec"),
        oss("rm"),
        oss(";"),
        oss("-x"),
        oss("foo"),
        oss("-exec"),
        oss("something"),
        oss(";"),
        oss("--exec"),
    ];

    let actual = transform_args_with_exec(original.into_iter());
    assert_eq!(expected, actual);
}

/// Parsing and size conversion tests
#[cfg(test)]
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
/// Parsing and size conversion tests data. Ensure that each type gets properly interpreted.
/// Call with higher base values to ensure expected multiplication (only need a couple)
#[cfg(test)]
gen_size_filter_parse_test! {
    parse_byte_plus: ("+1b", SizeFilter { size: 1, limit_type: SizeLimitType::Min, }),
    parse_byte_plus_multiplier: ("+10b", SizeFilter { size: 10, limit_type: SizeLimitType::Min, }),
    parse_byte_minus: ("-1b", SizeFilter { size: 1, limit_type: SizeLimitType::Max, }),
    parse_kilo_plus: ("+1k", SizeFilter { size: 1000, limit_type: SizeLimitType::Min, }),
    parse_kilo_plus_suffix: ("+1kb", SizeFilter { size: 1000, limit_type: SizeLimitType::Min, }),
    parse_kilo_minus: ("-1k", SizeFilter { size: 1000, limit_type: SizeLimitType::Max, }),
    parse_kilo_minus_multiplier: ("-100k", SizeFilter { size: 100000, limit_type: SizeLimitType::Max, }),
    parse_kilo_minus_suffix: ("-1kb", SizeFilter { size: 1000, limit_type: SizeLimitType::Max, }),
    parse_kilo_plus_upper: ("+1K", SizeFilter { size: 1000, limit_type: SizeLimitType::Min, }),
    parse_kilo_plus_suffix_upper: ("+1KB", SizeFilter { size: 1000, limit_type: SizeLimitType::Min, }),
    parse_kilo_minus_upper: ("-1K", SizeFilter { size: 1000, limit_type: SizeLimitType::Max, }),
    parse_kilo_minus_suffix_upper: ("-1Kb", SizeFilter { size: 1000, limit_type: SizeLimitType::Max, }),
    parse_kibi_plus: ("+1ki", SizeFilter { size: 1024, limit_type: SizeLimitType::Min, }),
    parse_kibi_plus_multiplier: ("+10ki", SizeFilter { size: 10240, limit_type: SizeLimitType::Min, }),
    parse_kibi_plus_suffix: ("+1kib", SizeFilter { size: 1024, limit_type: SizeLimitType::Min, }),
    parse_kibi_minus: ("-1ki", SizeFilter { size: 1024, limit_type: SizeLimitType::Max, }),
    parse_kibi_minus_multiplier: ("-100ki", SizeFilter { size: 102400, limit_type: SizeLimitType::Max, }),
    parse_kibi_minus_suffix: ("-1kib", SizeFilter { size: 1024, limit_type: SizeLimitType::Max, }),
    parse_kibi_plus_upper: ("+1KI", SizeFilter { size: 1024, limit_type: SizeLimitType::Min, }),
    parse_kibi_plus_suffix_upper: ("+1KiB", SizeFilter { size: 1024, limit_type: SizeLimitType::Min, }),
    parse_kibi_minus_upper: ("-1Ki", SizeFilter { size: 1024, limit_type: SizeLimitType::Max, }),
    parse_kibi_minus_suffix_upper: ("-1KIB", SizeFilter { size: 1024, limit_type: SizeLimitType::Max, }),
    parse_mega_plus: ("+1m", SizeFilter { size: 1000000, limit_type: SizeLimitType::Min, }),
    parse_mega_plus_suffix: ("+1mb", SizeFilter { size: 1000000, limit_type: SizeLimitType::Min, }),
    parse_mega_minus: ("-1m", SizeFilter { size: 1000000, limit_type: SizeLimitType::Max, }),
    parse_mega_minus_suffix: ("-1mb", SizeFilter { size: 1000000, limit_type: SizeLimitType::Max, }),
    parse_mega_plus_upper: ("+1M", SizeFilter { size: 1000000, limit_type: SizeLimitType::Min, }),
    parse_mega_plus_suffix_upper: ("+1MB", SizeFilter { size: 1000000, limit_type: SizeLimitType::Min, }),
    parse_mega_minus_upper: ("-1M", SizeFilter { size: 1000000, limit_type: SizeLimitType::Max, }),
    parse_mega_minus_suffix_upper: ("-1Mb", SizeFilter { size: 1000000, limit_type: SizeLimitType::Max, }),
    parse_mebi_plus: ("+1mi", SizeFilter { size: 1048576, limit_type: SizeLimitType::Min, }),
    parse_mebi_plus_suffix: ("+1mib", SizeFilter { size: 1048576, limit_type: SizeLimitType::Min, }),
    parse_mebi_minus: ("-1mi", SizeFilter { size: 1048576, limit_type: SizeLimitType::Max, }),
    parse_mebi_minus_suffix: ("-1mib", SizeFilter { size: 1048576, limit_type: SizeLimitType::Max, }),
    parse_mebi_plus_upper: ("+1MI", SizeFilter { size: 1048576, limit_type: SizeLimitType::Min, }),
    parse_mebi_plus_suffix_upper: ("+1MiB", SizeFilter { size: 1048576, limit_type: SizeLimitType::Min, }),
    parse_mebi_minus_upper: ("-1Mi", SizeFilter { size: 1048576, limit_type: SizeLimitType::Max, }),
    parse_mebi_minus_suffix_upper: ("-1MIB", SizeFilter { size: 1048576, limit_type: SizeLimitType::Max, }),
    parse_giga_plus: ("+1g", SizeFilter { size: 1000000000, limit_type: SizeLimitType::Min, }),
    parse_giga_plus_suffix: ("+1gb", SizeFilter { size: 1000000000, limit_type: SizeLimitType::Min, }),
    parse_giga_minus: ("-1g", SizeFilter { size: 1000000000, limit_type: SizeLimitType::Max, }),
    parse_giga_minus_suffix: ("-1gb", SizeFilter { size: 1000000000, limit_type: SizeLimitType::Max, }),
    parse_giga_plus_upper: ("+1G", SizeFilter { size: 1000000000, limit_type: SizeLimitType::Min, }),
    parse_giga_plus_suffix_upper: ("+1GB", SizeFilter { size: 1000000000, limit_type: SizeLimitType::Min, }),
    parse_giga_minus_upper: ("-1G", SizeFilter { size: 1000000000, limit_type: SizeLimitType::Max, }),
    parse_giga_minus_suffix_upper: ("-1Gb", SizeFilter { size: 1000000000, limit_type: SizeLimitType::Max, }),
    parse_gibi_plus: ("+1gi", SizeFilter { size: 1073741824, limit_type: SizeLimitType::Min, }),
    parse_gibi_plus_suffix: ("+1gib", SizeFilter { size: 1073741824, limit_type: SizeLimitType::Min, }),
    parse_gibi_minus: ("-1gi", SizeFilter { size: 1073741824, limit_type: SizeLimitType::Max, }),
    parse_gibi_minus_suffix: ("-1gib", SizeFilter { size: 1073741824, limit_type: SizeLimitType::Max, }),
    parse_gibi_plus_upper: ("+1GI", SizeFilter { size: 1073741824, limit_type: SizeLimitType::Min, }),
    parse_gibi_plus_suffix_upper: ("+1GiB", SizeFilter { size: 1073741824, limit_type: SizeLimitType::Min, }),
    parse_gibi_minus_upper: ("-1Gi", SizeFilter { size: 1073741824, limit_type: SizeLimitType::Max, }),
    parse_gibi_minus_suffix_upper: ("-1GIB", SizeFilter { size: 1073741824, limit_type: SizeLimitType::Max, }),
    parse_tera_plus: ("+1t", SizeFilter { size: 1000000000000, limit_type: SizeLimitType::Min, }),
    parse_tera_plus_suffix: ("+1tb", SizeFilter { size: 1000000000000, limit_type: SizeLimitType::Min, }),
    parse_tera_minus: ("-1t", SizeFilter { size: 1000000000000, limit_type: SizeLimitType::Max, }),
    parse_tera_minus_suffix: ("-1tb", SizeFilter { size: 1000000000000, limit_type: SizeLimitType::Max, }),
    parse_tera_plus_upper: ("+1T", SizeFilter { size: 1000000000000, limit_type: SizeLimitType::Min, }),
    parse_tera_plus_suffix_upper: ("+1TB", SizeFilter { size: 1000000000000, limit_type: SizeLimitType::Min, }),
    parse_tera_minus_upper: ("-1T", SizeFilter { size: 1000000000000, limit_type: SizeLimitType::Max, }),
    parse_tera_minus_suffix_upper: ("-1Tb", SizeFilter { size: 1000000000000, limit_type: SizeLimitType::Max, }),
    parse_tebi_plus: ("+1ti", SizeFilter { size: 1099511627776, limit_type: SizeLimitType::Min, }),
    parse_tebi_plus_suffix: ("+1tib", SizeFilter { size: 1099511627776, limit_type: SizeLimitType::Min, }),
    parse_tebi_minus: ("-1ti", SizeFilter { size: 1099511627776, limit_type: SizeLimitType::Max, }),
    parse_tebi_minus_suffix: ("-1tib", SizeFilter { size: 1099511627776, limit_type: SizeLimitType::Max, }),
    parse_tebi_plus_upper: ("+1TI", SizeFilter { size: 1099511627776, limit_type: SizeLimitType::Min, }),
    parse_tebi_plus_suffix_upper: ("+1TiB", SizeFilter { size: 1099511627776, limit_type: SizeLimitType::Min, }),
    parse_tebi_minus_upper: ("-1Ti", SizeFilter { size: 1099511627776, limit_type: SizeLimitType::Max, }),
    parse_tebi_minus_suffix_upper: ("-1TIB", SizeFilter { size: 1099511627776, limit_type: SizeLimitType::Max, }),
}

/// Invalid parse testing
#[cfg(test)]
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

/// Invalid parse data
#[cfg(test)]
gen_size_filter_failure! {
    ensure_missing_symbol_returns_none: "10M",
    ensure_missing_number_returns_none: "+g",
    ensure_missing_unit_returns_none: "+18",
    ensure_bad_format_returns_none_1: "$10M",
    ensure_bad_format_returns_none_2: "badval",
    ensure_bad_format_returns_none_3: "9999",
    ensure_invalid_unit_returns_none_1: "+50a",
    ensure_invalid_unit_returns_none_2: "-10v",
    ensure_invalid_unit_returns_none_3: "+1Mv",
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

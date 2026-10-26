use std::ffi::OsStr;

use anyhow::{Result, anyhow};
use globset::{Glob, GlobBuilder, GlobMatcher, GlobSet, GlobSetBuilder};
use memchr::memmem;
use regex::bytes::{RegexSet, RegexSetBuilder};

pub trait Matcher {
    fn matches_path(&self, path: &OsStr) -> bool;
}

pub type Patterns = Box<dyn Matcher + Send + Sync>;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum PatternType {
    Regex,
    Fixed,
    Glob,
    Exact,
}

impl Matcher for RegexSet {
    fn matches_path(&self, path: &OsStr) -> bool {
        let haystack = path.as_encoded_bytes();
        let matches = self.matches(haystack);
        // Return true if the number of regexes that matched
        // equals the total number of regexes.
        matches.iter().count() == self.len()
    }
}

impl Matcher for GlobSet {
    fn matches_path(&self, path: &OsStr) -> bool {
        self.matches_all(path)
    }
}

/// In the common case a single glob, it is simpler, and
/// faster to just use a single Glob instead of a GlobSet
impl Matcher for GlobMatcher {
    fn matches_path(&self, path: &OsStr) -> bool {
        self.is_match(path)
    }
}

/// Matcher that matches fixed strings
pub struct FixedStrings(pub Vec<String>);

impl Matcher for FixedStrings {
    fn matches_path(&self, path: &OsStr) -> bool {
        let path = path.as_encoded_bytes();
        self.0.iter().all(|f| bytes_contains(path, f.as_bytes()))
    }
}

pub struct ExactStrings(pub Vec<String>);

impl Matcher for ExactStrings {
    fn matches_path(&self, path: &OsStr) -> bool {
        self.0.iter().all(|s| s.as_str() == path)
    }
}

/// Matcher that matches everything
pub struct MatchAll;
impl Matcher for MatchAll {
    fn matches_path(&self, _path: &OsStr) -> bool {
        true
    }
}
pub fn build_patterns(
    mut patterns: Vec<String>,
    pattern_type: PatternType,
    ignore_case: bool,
) -> Result<Patterns> {
    if patterns.is_empty() || is_empty_pattern(&patterns, pattern_type) {
        return Ok(Box::new(MatchAll));
    }
    match pattern_type {
        PatternType::Glob => build_glob_matcher(patterns, ignore_case),
        PatternType::Fixed if !ignore_case => Ok(Box::new(FixedStrings(patterns))),
        PatternType::Exact if !ignore_case => Ok(Box::new(ExactStrings(patterns))),
        typ => {
            // TODO: is there a better way we could handle case insensitive fixed strings?
            if typ == PatternType::Fixed {
                for pattern in patterns.iter_mut() {
                    *pattern = regex::escape(pattern);
                }
            } else if typ == PatternType::Exact {
                // Would using eq_ignore_ascii case be good enough?
                for pattern in patterns.iter_mut() {
                    // Anchor the escaped pattern so the full filename (or path) must match exactly.
                    // Literal. No substring matching.
                    *pattern = format!("^{}$", regex::escape(pattern));
                }
            }
            Ok(Box::new(build_regex_matcher(patterns, ignore_case)?))
        }
    }
}

fn build_glob_matcher(patterns: Vec<String>, ignore_case: bool) -> Result<Patterns> {
    Ok(if patterns.len() == 1 {
        Box::new(build_glob(&patterns[0], ignore_case)?.compile_matcher())
    } else {
        let mut builder = GlobSetBuilder::new();
        for pat in patterns {
            builder.add(build_glob(&pat, ignore_case)?);
        }
        Box::new(builder.build()?)
    })
}

fn build_glob(pattern: &str, ignore_case: bool) -> Result<Glob> {
    Ok(GlobBuilder::new(pattern)
        .literal_separator(true)
        .case_insensitive(ignore_case)
        .build()?)
}

fn build_regex_matcher(patterns: Vec<String>, ignore_case: bool) -> Result<RegexSet> {
    RegexSetBuilder::new(patterns)
        .case_insensitive(ignore_case)
        .dot_matches_new_line(true)
        .build()
        .map_err(|e| {
            anyhow!(
                "{}\n\nNote: You can search for literal substrings with '--fixed-strings' \
                 or literal strings with '--exact' options (instead of a regular expression). \
                 Alternatively, you can also use the '--glob' option to match on a glob pattern.",
                e,
            )
        })
}

fn is_empty_pattern(patterns: &[String], pattern_type: PatternType) -> bool {
    pattern_type != PatternType::Exact && patterns.len() == 1 && patterns[0].is_empty()
}

/// Test if the needle is a substring of the haystack
fn bytes_contains(haystack: &[u8], needle: &[u8]) -> bool {
    memmem::find(haystack, needle).is_some()
}

#[test]
fn empty_matches_all() {
    let patterns_none = build_patterns(Vec::new(), PatternType::Glob, false).unwrap();
    let patterns_empty_str =
        build_patterns(vec!["".to_string()], PatternType::Glob, false).unwrap();

    assert!(patterns_none.matches_path("abcd".as_ref()));
    assert!(patterns_empty_str.matches_path("abcd".as_ref()));
}

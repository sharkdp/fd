use std::cell::RefCell;
use std::path::Path;

use anyhow::{anyhow, Result};
use globset::{Glob, GlobBuilder, GlobMatcher, GlobSet, GlobSetBuilder};
use memchr::memmem;
use regex::bytes::{RegexSet, RegexSetBuilder};

pub trait Matcher {
    fn matches_path(&self, path: &Path) -> bool;
}

pub type Patterns = Box<dyn Matcher + Send + Sync>;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum PatternType {
    Regex,
    Fixed,
    Glob,
    #[cfg(feature = "pcre")]
    Pcre,
}

impl Matcher for RegexSet {
    fn matches_path(&self, path: &Path) -> bool {
        let haystack = path.as_os_str().as_encoded_bytes();
        let matches = self.matches(haystack);
        // Return true if the number of regexes that matched
        // equals the total number of regexes.
        matches.iter().count() == self.len()
    }
}

#[cfg(feature = "pcre")]
impl Matcher for Vec<pcre2::bytes::Regex> {
    fn matches_path(&self, path: &Path) -> bool {
        let path = path.as_os_str().as_encoded_bytes();
        self.iter().all(|pat| pat.is_match(path).unwrap())
    }
}

thread_local! {
    /// Thread local cache for Vec to use for globset matches
    static GLOB_MATCHES: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}
impl Matcher for GlobSet {
    fn matches_path(&self, path: &Path) -> bool {
        GLOB_MATCHES.with_borrow_mut(|matches| {
            self.matches_into(path, matches);
            matches.len() == self.len()
        })
    }
}

/// In the common case a single glob, it is simpler, and
/// faster to just use a single Glob instead of a GlobSet
impl Matcher for GlobMatcher {
    fn matches_path(&self, path: &Path) -> bool {
        self.is_match(path)
    }
}

/// Matcher that matches fixed strings
pub struct FixedStrings(pub Vec<String>);

impl Matcher for FixedStrings {
    fn matches_path(&self, path: &Path) -> bool {
        let path = path.as_os_str().as_encoded_bytes();
        self.0.iter().all(|f| bytes_contains(path, f.as_bytes()))
    }
}

/// Matcher that matches everything
pub struct MatchAll;
impl Matcher for MatchAll {
    fn matches_path(&self, _path: &Path) -> bool {
        true
    }
}
pub fn build_patterns(
    mut patterns: Vec<String>,
    pattern_type: PatternType,
    ignore_case: bool,
) -> Result<Patterns> {
    if patterns.is_empty() {
        return Ok(Box::new(MatchAll));
    }
    match pattern_type {
        PatternType::Glob => build_glob_matcher(patterns, ignore_case),
        #[cfg(feature = "pcre")]
        PatternType::Pcre => Ok(Box::new(build_pcre_matcher(patterns, ignore_case)?)),
        PatternType::Fixed if !ignore_case => Ok(Box::new(FixedStrings(patterns))),
        typ => {
            // TODO: is there a better way we could handle case insensitive fixed strings?
            if typ == PatternType::Fixed {
                for pattern in patterns.iter_mut() {
                    *pattern = regex::escape(pattern);
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

// Should we enable the unicde/utf8 features for regex and pcre?

#[cfg(feature = "pcre")]
fn build_pcre_matcher(
    patterns: Vec<String>,
    ignore_case: bool,
) -> Result<Vec<pcre2::bytes::Regex>> {
    use pcre2::bytes::RegexBuilder;
    patterns
        .iter()
        .map(|pat| {
            RegexBuilder::new()
                .dotall(true)
                .caseless(ignore_case)
                .build(pat)
                .map_err(|e| {
                    anyhow!(
                        "{}\n\nNote: You can use the '--fixed-strings' option to search for a \
                 literal string instead of a regular expression. Alternatively, you can \
                 also use the '--glob' option to match on a glob pattern.",
                        e.to_string()
                    )
                })
        })
        .collect()
}

#[cfg(feature = "pcre")]
const PCRE_ALT_MSG: &str = " Use --pcre to enable perl-compatible regex features.";
#[cfg(not(feature = "pcre"))]
const PCRE_ALT_MSG: &str = "";

fn build_regex_matcher(patterns: Vec<String>, ignore_case: bool) -> Result<RegexSet> {
    RegexSetBuilder::new(patterns)
        .case_insensitive(ignore_case)
        .dot_matches_new_line(true)
        .build()
        .map_err(|e| {
            anyhow!(
                "{}\n\nNote: You can use the '--fixed-strings' option to search for a \
                 literal string instead of a regular expression. Alternatively, you can \
                 also use the '--glob' option to match on a glob pattern.{}",
                e.to_string(),
                PCRE_ALT_MSG
            )
        })
}

/// Test if the needle is a substring of the haystack
fn bytes_contains(haystack: &[u8], needle: &[u8]) -> bool {
    memmem::find(haystack, needle).is_some()
}

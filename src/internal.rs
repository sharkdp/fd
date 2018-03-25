// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::ffi::OsString;
use std::process;
use std::time;
use std::io::Write;
use std::fs;
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::PermissionsExt;

use exec::CommandTemplate;
use lscolors::LsColors;
use regex_syntax::Parser;
use regex_syntax::hir::Hir;
use regex::RegexSet;

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

#[cfg(any(unix, target_os = "redox"))]
pub fn is_executable(md: &fs::Metadata) -> bool {
    md.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
pub fn is_executable(_: &fs::Metadata) -> bool {
    false
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

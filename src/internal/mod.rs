// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use regex_syntax::{hir::Hir, Parser};
use std::{ffi::OsString, process};

pub use self::file_types::FileTypes;
pub use self::size_filter::SizeFilter;

mod file_types;
mod size_filter;

/// Print error message to stderr.
pub fn print_error(message: &str) {
    eprintln!("{}", message);
}

/// Print error message to stderr and exit with status `1`.
pub fn print_error_and_exit(message: &str) -> ! {
    print_error(message);
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
mod tests {
    use super::*;

    macro_rules! oss_vec {
        ($($x:expr),*) => {
            vec![$(OsString::from($x)),*]
        };
        ($($x:expr),*,) => (oss_vec![$($x),*])
    }

    /// Ensure that -exec gets transformed into --exec
    #[test]
    fn normal_exec_substitution() {
        let original = oss_vec!["fd", "foo", "-exec", "cmd"];
        let expected = oss_vec!["fd", "foo", "--exec", "cmd"];

        let actual = transform_args_with_exec(original.into_iter());
        assert_eq!(expected, actual);
    }

    /// Ensure that --exec is not touched
    #[test]
    fn passthru_of_original_exec() {
        let original = oss_vec!["fd", "foo", "--exec", "cmd"];
        let expected = oss_vec!["fd", "foo", "--exec", "cmd"];

        let actual = transform_args_with_exec(original.into_iter());
        assert_eq!(expected, actual);
    }

    #[test]
    fn temp_check_that_exec_context_observed() {
        let original = oss_vec![
            "fd",
            "foo",
            "-exec",
            "cmd",
            "-exec",
            "ls",
            ";",
            "-exec",
            "rm",
            ";",
            "--exec",
            "find",
            "-exec",
            "rm",
            ";",
            "-x",
            "foo",
            "-exec",
            "something",
            ";",
            "-exec",
        ];
        let expected = oss_vec![
            "fd",
            "foo",
            "--exec",
            "cmd",
            "-exec",
            "ls",
            ";",
            "--exec",
            "rm",
            ";",
            "--exec",
            "find",
            "-exec",
            "rm",
            ";",
            "-x",
            "foo",
            "-exec",
            "something",
            ";",
            "--exec",
        ];

        let actual = transform_args_with_exec(original.into_iter());
        assert_eq!(expected, actual);
    }
}

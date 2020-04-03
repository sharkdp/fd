use std::borrow::Cow;
use std::ffi::{OsStr, OsString};

use regex_syntax::hir::Hir;
use regex_syntax::ParserBuilder;

#[cfg(any(unix, target_os = "redox"))]
pub fn osstr_to_bytes(input: &OsStr) -> Cow<[u8]> {
    use std::os::unix::ffi::OsStrExt;
    Cow::Borrowed(input.as_bytes())
}

#[cfg(windows)]
pub fn osstr_to_bytes(input: &OsStr) -> Cow<[u8]> {
    let string = input.to_string_lossy();

    match string {
        Cow::Owned(string) => Cow::Owned(string.into_bytes()),
        Cow::Borrowed(string) => Cow::Borrowed(string.as_bytes()),
    }
}

/// Determine if a regex pattern contains a literal uppercase character.
pub fn pattern_has_uppercase_char(pattern: &str) -> bool {
    let mut parser = ParserBuilder::new().allow_invalid_utf8(true).build();

    parser
        .parse(pattern)
        .map(|hir| hir_has_uppercase_char(&hir))
        .unwrap_or(false)
}

/// Determine if a regex expression contains a literal uppercase character.
fn hir_has_uppercase_char(hir: &Hir) -> bool {
    use regex_syntax::hir::*;

    match *hir.kind() {
        HirKind::Literal(Literal::Unicode(c)) => c.is_uppercase(),
        HirKind::Literal(Literal::Byte(b)) => char::from(b).is_uppercase(),
        HirKind::Class(Class::Unicode(ref ranges)) => ranges
            .iter()
            .any(|r| r.start().is_uppercase() || r.end().is_uppercase()),
        HirKind::Class(Class::Bytes(ref ranges)) => ranges
            .iter()
            .any(|r| char::from(r.start()).is_uppercase() || char::from(r.end()).is_uppercase()),
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

    fn oss_vec(strs: &[&str]) -> Vec<OsString> {
        strs.into_iter().map(OsString::from).collect()
    }

    /// Ensure that -exec gets transformed into --exec
    #[test]
    fn normal_exec_substitution() {
        let original = oss_vec(&["fd", "foo", "-exec", "cmd"]);
        let expected = oss_vec(&["fd", "foo", "--exec", "cmd"]);
        let actual = transform_args_with_exec(original.into_iter());
        assert_eq!(expected, actual);
    }
    /// Ensure that --exec is not touched
    #[test]
    fn passthru_of_original_exec() {
        let original = oss_vec(&["fd", "foo", "--exec", "cmd"]);
        let expected = oss_vec(&["fd", "foo", "--exec", "cmd"]);
        let actual = transform_args_with_exec(original.into_iter());
        assert_eq!(expected, actual);
    }
    #[test]
    fn temp_check_that_exec_context_observed() {
        let original = oss_vec(&[
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
        ]);
        let expected = oss_vec(&[
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
        ]);
        let actual = transform_args_with_exec(original.into_iter());
        assert_eq!(expected, actual);
    }
}

use regex_syntax::hir::Hir;
use regex_syntax::ParserBuilder;

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

    match hir.kind() {
        HirKind::Literal(Literal::Unicode(c)) => c.is_uppercase(),
        HirKind::Literal(Literal::Byte(b)) => char::from(*b).is_uppercase(),
        HirKind::Class(Class::Unicode(ranges)) => ranges
            .iter()
            .any(|r| r.start().is_uppercase() || r.end().is_uppercase()),
        HirKind::Class(Class::Bytes(ranges)) => ranges
            .iter()
            .any(|r| char::from(r.start()).is_uppercase() || char::from(r.end()).is_uppercase()),
        HirKind::Group(Group { hir, .. }) | HirKind::Repetition(Repetition { hir, .. }) => {
            hir_has_uppercase_char(hir)
        }
        HirKind::Concat(hirs) | HirKind::Alternation(hirs) => {
            hirs.iter().any(hir_has_uppercase_char)
        }
        _ => false,
    }
}

/// Determine if a regex pattern only matches strings starting with a literal dot (hidden files)
pub fn pattern_matches_strings_with_leading_dot(pattern: &str) -> bool {
    let mut parser = ParserBuilder::new().allow_invalid_utf8(true).build();

    parser
        .parse(pattern)
        .map(|hir| hir_matches_strings_with_leading_dot(&hir))
        .unwrap_or(false)
}

/// See above.
fn hir_matches_strings_with_leading_dot(hir: &Hir) -> bool {
    use regex_syntax::hir::*;

    // Note: this only really detects the simplest case where a regex starts with
    // "^\\.", i.e. a start text anchor and a literal dot character. There are a lot
    // of other patterns that ONLY match hidden files, e.g. ^(\\.foo|\\.bar) which are
    // not (yet) detected by this algorithm.
    match hir.kind() {
        HirKind::Concat(hirs) => {
            let mut hirs = hirs.iter();
            if let Some(hir) = hirs.next() {
                if hir.kind() != &HirKind::Anchor(Anchor::StartText) {
                    return false;
                }
            } else {
                return false;
            }

            if let Some(hir) = hirs.next() {
                hir.kind() == &HirKind::Literal(Literal::Unicode('.'))
            } else {
                false
            }
        }
        _ => false,
    }
}

#[test]
fn pattern_has_uppercase_char_simple() {
    assert!(pattern_has_uppercase_char("A"));
    assert!(pattern_has_uppercase_char("foo.EXE"));

    assert!(!pattern_has_uppercase_char("a"));
    assert!(!pattern_has_uppercase_char("foo.exe123"));
}

#[test]
fn pattern_has_uppercase_char_advanced() {
    assert!(pattern_has_uppercase_char("foo.[a-zA-Z]"));

    assert!(!pattern_has_uppercase_char(r"\Acargo"));
    assert!(!pattern_has_uppercase_char(r"carg\x6F"));
}

#[test]
fn matches_strings_with_leading_dot_simple() {
    assert!(pattern_matches_strings_with_leading_dot("^\\.gitignore"));

    assert!(!pattern_matches_strings_with_leading_dot("^.gitignore"));
    assert!(!pattern_matches_strings_with_leading_dot("\\.gitignore"));
    assert!(!pattern_matches_strings_with_leading_dot("^gitignore"));
}

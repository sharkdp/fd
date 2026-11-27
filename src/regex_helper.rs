use regex_syntax::ParserBuilder;
use regex_syntax::hir::Hir;

/// Determine if a regex pattern contains a literal uppercase character.
pub fn pattern_has_uppercase_char(pattern: &str) -> bool {
    // The HIR includes case-folded alternatives introduced by inline flags.
    // Smart case must consider the original literals, before case folding.
    regex_syntax::ast::parse::Parser::new()
        .parse(pattern)
        .map(|ast| regex_syntax::ast::visit(&ast, UppercaseVisitor(false)).unwrap_or(false))
        .unwrap_or(false)
}

struct UppercaseVisitor(bool);

impl regex_syntax::ast::Visitor for UppercaseVisitor {
    type Output = bool;
    type Err = std::convert::Infallible;

    fn finish(self) -> Result<bool, Self::Err> {
        Ok(self.0)
    }

    fn visit_pre(&mut self, ast: &regex_syntax::ast::Ast) -> Result<(), Self::Err> {
        if let regex_syntax::ast::Ast::Literal(literal) = ast {
            self.0 |= literal.c.is_uppercase();
        }
        Ok(())
    }

    fn visit_class_set_item_pre(
        &mut self,
        item: &regex_syntax::ast::ClassSetItem,
    ) -> Result<(), Self::Err> {
        use regex_syntax::ast::ClassSetItem;

        self.0 |= match item {
            ClassSetItem::Literal(literal) => literal.c.is_uppercase(),
            ClassSetItem::Range(range) => {
                range.start.c.is_uppercase() || range.end.c.is_uppercase()
            }
            _ => false,
        };
        Ok(())
    }
}

/// Determine if a regex pattern only matches strings starting with a literal dot (hidden files)
pub fn pattern_matches_strings_with_leading_dot(pattern: &str) -> bool {
    let mut parser = ParserBuilder::new().utf8(false).build();

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
                if hir.kind() != &HirKind::Look(Look::Start) {
                    return false;
                }
            } else {
                return false;
            }

            if let Some(hir) = hirs.next() {
                match hir.kind() {
                    HirKind::Literal(Literal(bytes)) => bytes.starts_with(b"."),
                    _ => false,
                }
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

#[test]
fn pattern_has_uppercase_char_inline_flags() {
    for pattern in [
        "(?i)foo",
        "(?i:foo)",
        "(?i)[a-z]",
        "(?i)[a-z&&[^q]]",
        "(?i)é",
    ] {
        assert!(!pattern_has_uppercase_char(pattern), "{pattern}");
    }
    for pattern in ["(?i)Foo", "(?i)[A-Z]", r"(?i)\x46oo", "(?i)É"] {
        assert!(pattern_has_uppercase_char(pattern), "{pattern}");
    }
}

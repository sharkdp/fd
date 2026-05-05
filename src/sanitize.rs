//! TTY-output sanitization to prevent terminal escape injection via filenames.

use std::borrow::Cow;
use std::fmt::{self, Display, Formatter, Write};

/// True for any char that is neither printable nor permitted whitespace (only HT).
/// Covers C0/C1/DEL, bidi overrides, zero-width and format chars, and tag chars.
#[inline]
fn needs_escape(c: char) -> bool {
    if c == '\t' {
        return false;
    }
    c.is_control()
        || matches!(c,
            '\u{00AD}'                  // soft hyphen (invisible)
            | '\u{180E}'                // Mongolian vowel separator
            | '\u{200B}'..='\u{200F}'   // zero-width + LRM/RLM
            | '\u{202A}'..='\u{202E}'   // bidi embedding/override
            | '\u{2060}'..='\u{206F}'   // word joiner, invisibles, deprecated formats
            | '\u{FEFF}'                // BOM / zero-width no-break space
            | '\u{FFF9}'..='\u{FFFB}'   // interlinear annotation
            | '\u{E0000}'..='\u{E007F}' // language tags
        )
}

/// Streams `s` to a formatter, escaping dangerous chars as `\xNN` / `\u{NNNN}`.
/// Allocation-free wrapper for use with `write!`, `format!`, etc.
pub struct Sanitized<'a>(pub &'a str);

impl Display for Sanitized<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for c in self.0.chars() {
            if needs_escape(c) {
                let v = c as u32;
                if v <= 0xFF {
                    write!(f, "\\x{v:02X}")?;
                } else {
                    write!(f, "\\u{{{v:04X}}}")?;
                }
            } else {
                f.write_char(c)?;
            }
        }
        Ok(())
    }
}

/// Returns a `Cow<str>` borrowing `s` when no escaping is needed, otherwise an owned
/// escaped copy. Use this when an owned `&str`/`String` is required (e.g. ANSI paint).
pub fn sanitize_for_terminal(s: &str) -> Cow<'_, str> {
    if !s.chars().any(needs_escape) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if needs_escape(c) {
            let v = c as u32;
            if v <= 0xFF {
                let _ = write!(out, "\\x{v:02X}");
            } else {
                let _ = write!(out, "\\u{{{v:04X}}}");
            }
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
}

/// Sanitize for terminal output only; raw bytes pass through on pipes/files.
pub fn maybe_sanitize<'a>(s: &'a str, is_terminal: bool) -> Cow<'a, str> {
    if is_terminal {
        sanitize_for_terminal(s)
    } else {
        Cow::Borrowed(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_safe_content() {
        for s in [
            "hello.txt",
            "résumé.pdf",
            "文档.txt",
            "🦀.rs",
            "a\tb",
            "a\u{FFFD}b",
        ] {
            assert!(
                matches!(sanitize_for_terminal(s), Cow::Borrowed(_)),
                "{s:?}"
            );
            assert_eq!(sanitize_for_terminal(s), s);
            // Display matches Cow output.
            assert_eq!(Sanitized(s).to_string(), s);
        }
    }

    #[test]
    fn strips_osc52_clipboard_payload() {
        let attack = "innocent\x1b]52;c;cHduZWQ=\x1b\\.txt";
        let safe = sanitize_for_terminal(attack);
        assert!(!safe.contains('\x1b'));
        assert_eq!(safe, "innocent\\x1B]52;c;cHduZWQ=\\x1B\\.txt");
    }

    #[test]
    fn strips_cr_output_forgery() {
        assert_eq!(sanitize_for_terminal("A\rFAKE OUTPUT"), "A\\x0DFAKE OUTPUT");
    }

    #[test]
    fn strips_osc8_hyperlink_injection() {
        let attack = "phish\x1b]8;;https:evil.example\x1b\\phony.txt";
        assert!(!sanitize_for_terminal(attack).contains('\x1b'));
    }

    #[test]
    fn strips_del() {
        assert_eq!(sanitize_for_terminal("a\x7fb"), "a\\x7Fb");
    }

    #[test]
    fn strips_bel_and_null() {
        assert_eq!(sanitize_for_terminal("a\x07b"), "a\\x07b");
        assert_eq!(sanitize_for_terminal("a\0b"), "a\\x00b");
    }

    #[test]
    fn strips_newline() {
        assert_eq!(sanitize_for_terminal("a\nb"), "a\\x0Ab");
    }

    #[test]
    fn escape_preserves_information() {
        let s = "name\x1bX\x07Y.txt";
        assert_eq!(sanitize_for_terminal(s), "name\\x1BX\\x07Y.txt");
    }

    #[test]
    fn strips_c1_csi_and_osc_initiators() {
        // U+009B is CSI, U+009D is OSC; dangerous on 8-bit-control terminals.
        assert_eq!(sanitize_for_terminal("\u{9b}31m"), "\\x9B31m");
        assert_eq!(
            sanitize_for_terminal("\u{9d}0;pwned\u{9c}"),
            "\\x9D0;pwned\\x9C"
        );
    }

    #[test]
    fn strips_bidi_overrides_and_zero_width() {
        // Trojan-Source style RLO/LRO that flip rendered order of filename text.
        assert_eq!(
            sanitize_for_terminal("safe\u{202E}fil\u{202D}gnp.exe"),
            "safe\\u{202E}fil\\u{202D}gnp.exe"
        );
        // Zero-width space and BOM are also format chars used to disguise filenames.
        assert_eq!(sanitize_for_terminal("a\u{200B}b"), "a\\u{200B}b");
        assert_eq!(sanitize_for_terminal("\u{FEFF}name"), "\\u{FEFF}name");
    }

    #[test]
    fn keeps_legitimate_unicode_features() {
        // Variation selectors (U+FE0F, U+E0100..) modify preceding glyphs in CJK/emoji
        // and are legitimate in filenames. Private-use chars are used by icon fonts.
        for s in [
            "heart\u{2764}\u{FE0F}.txt",
            "icon\u{E000}.cfg",
            "cjk\u{6F22}\u{E0101}.txt",
        ] {
            assert!(
                matches!(sanitize_for_terminal(s), Cow::Borrowed(_)),
                "{s:?}"
            );
        }
    }

    #[test]
    fn display_streams_without_intermediate_string() {
        // Sanitized<'_> implements Display directly; produces same bytes as Cow form.
        let attack = "x\x1byb";
        let mut out = String::new();
        write!(out, "{}", Sanitized(attack)).unwrap();
        assert_eq!(out, "x\\x1Byb");
    }

    #[test]
    fn maybe_sanitize_passthrough_when_not_terminal() {
        let attack = "x\x1by";
        // Pipe context: bytes pass through unchanged (zero-copy).
        let out = maybe_sanitize(attack, false);
        assert!(matches!(out, Cow::Borrowed(_)));
        assert_eq!(out, attack);
        // TTY context: escapes apply.
        assert_eq!(maybe_sanitize(attack, true), "x\\x1By");
    }
}

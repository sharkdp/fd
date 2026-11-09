//! TTY-output sanitization to prevent terminal escape injection via filenames.

use std::fmt::{Display, Formatter, Write};

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

/// Write the sanitized contents of `raw` to `f`.
pub fn write_sanitized(f: &mut std::fmt::Formatter<'_>, raw: &str) -> std::fmt::Result {
    // Would it be faster to do a pass to see if we don't need an escape and write
    // the whole string first? Maybe with a faster check just for non-control ASCII?
    for c in raw.chars() {
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

// TODO: add something to sanitize paths directly instead of as strings.

/// A wrapper type that sanitizes the output to Display
pub(crate) struct SanitizedStr<'a> {
    raw: &'a str,
    /// If true, the content is safe to output without sanitizing,
    /// either because it is trusted, or because it isn't going to a terminal.
    is_safe: bool,
}

impl<'a> Display for SanitizedStr<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_safe {
            // We don't need to sanitize anything, just forward
            self.raw.fmt(f)
        } else {
            write_sanitized(f, self.raw)
        }
    }
}

/// Sanitize for terminal output only; raw bytes pass through on pipes/files.
pub fn sanitize_for_term(raw: &str, is_terminal: bool) -> SanitizedStr<'_> {
    SanitizedStr {
        raw,
        is_safe: !is_terminal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sanitize_string(s: &str) -> String {
        SanitizedStr {
            raw: s,
            is_safe: false,
        }
        .to_string()
    }

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
            assert_eq!(sanitize_string(s), s);
        }
    }

    #[test]
    fn strips_osc52_clipboard_payload() {
        let attack = "innocent\x1b]52;c;cHduZWQ=\x1b\\.txt";
        let safe = sanitize_string(attack);
        assert!(!safe.contains('\x1b'));
        assert_eq!(safe, "innocent\\x1B]52;c;cHduZWQ=\\x1B\\.txt");
    }

    #[test]
    fn strips_cr_output_forgery() {
        assert_eq!(sanitize_string("A\rFAKE OUTPUT"), "A\\x0DFAKE OUTPUT");
    }

    #[test]
    fn strips_osc8_hyperlink_injection() {
        let attack = "phish\x1b]8;;https:evil.example\x1b\\phony.txt";
        assert!(!sanitize_string(attack).contains('\x1b'));
    }

    #[test]
    fn strips_del() {
        assert_eq!(sanitize_string("a\x7fb"), "a\\x7Fb");
    }

    #[test]
    fn strips_bel_and_null() {
        assert_eq!(sanitize_string("a\x07b"), "a\\x07b");
        assert_eq!(sanitize_string("a\0b"), "a\\x00b");
    }

    #[test]
    fn escape_preserves_information() {
        let s = "name\x1bX\x07Y.txt";
        assert_eq!(sanitize_string(s), "name\\x1BX\\x07Y.txt");
    }

    #[test]
    fn strips_c1_csi_and_osc_initiators() {
        // U+009B is CSI, U+009D is OSC; dangerous on 8-bit-control terminals.
        assert_eq!(sanitize_string("\u{9b}31m"), "\\x9B31m");
        assert_eq!(sanitize_string("\u{9d}0;pwned\u{9c}"), "\\x9D0;pwned\\x9C");
    }

    #[test]
    fn strips_bidi_overrides_and_zero_width() {
        // Trojan-Source style RLO/LRO that flip rendered order of filename text.
        assert_eq!(
            sanitize_string("safe\u{202E}fil\u{202D}gnp.exe"),
            "safe\\u{202E}fil\\u{202D}gnp.exe"
        );
        // Zero-width space and BOM are also format chars used to disguise filenames.
        assert_eq!(sanitize_string("a\u{200B}b"), "a\\u{200B}b");
        assert_eq!(sanitize_string("\u{FEFF}name"), "\\u{FEFF}name");
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
            assert_eq!(sanitize_string(s), s);
        }
    }

    #[test]
    fn maybe_sanitize_passthrough_when_not_terminal() {
        let attack = "x\x1by";
        // Pipe context: bytes pass through unchanged (zero-copy).
        let out = sanitize_for_term(attack, false).to_string();
        assert_eq!(out, attack);
        // TTY context: escapes apply.
        assert_eq!(sanitize_for_term(attack, true).to_string(), "x\\x1By");
    }
}

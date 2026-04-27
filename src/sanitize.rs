//! TTY-output sanitization to prevent terminal escape injection via filenames.

use std::borrow::Cow;
use std::fmt::Write;

#[inline]
fn is_dangerous_control(c: char) -> bool {
    // C0 (except HT), DEL, and C1 controls (U+0080..=U+009F can act as
    // single-byte CSI/OSC initiators on 8-bit-control terminals).
    matches!(c, '\x00'..='\x08' | '\x0A'..='\x1F' | '\x7F' | '\u{80}'..='\u{9F}')
}

/// Replace control characters with `\xNN` so the original filename remains recoverable.
pub fn sanitize_for_terminal(s: &str) -> Cow<'_, str> {
    if !s.chars().any(is_dangerous_control) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if is_dangerous_control(c) {
            let _ = write!(out, "\\x{:02X}", c as u32);
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
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
}

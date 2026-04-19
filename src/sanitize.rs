//! TTY-output sanitization to prevent terminal escape injection via filenames.

use std::borrow::Cow;

#[inline]
fn is_dangerous_control(c: char) -> bool {
    // C0 (except HT), DEL, and C1 controls (gap fix: U+0080..=U+009F can act
    // as single-byte CSI/OSC initiators on 8-bit-control terminals).
    matches!(c, '\x00'..='\x08' | '\x0A'..='\x1F' | '\x7F' | '\u{80}'..='\u{9F}')
}

/// Replace control characters with `?` (matches `ls -q`). Borrows on the fast path.
pub fn sanitize_for_terminal(s: &str) -> Cow<'_, str> {
    if !s.chars().any(is_dangerous_control) {
        return Cow::Borrowed(s);
    }
    Cow::Owned(
        s.chars()
            .map(|c| if is_dangerous_control(c) { '?' } else { c })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_safe_content() {
        // Plain text, unicode, tab, and existing U+FFFD all pass through without allocation.
        for s in ["hello.txt", "résumé.pdf", "文档.txt", "🦀.rs", "a\tb", "a\u{FFFD}b"] {
            assert!(matches!(sanitize_for_terminal(s), Cow::Borrowed(_)), "{s:?}");
            assert_eq!(sanitize_for_terminal(s), s);
        }
    }

    #[test]
    fn strips_osc52_clipboard_payload() {
        let attack = "innocent\x1b]52;c;cHduZWQ=\x1b\\.txt";
        let safe = sanitize_for_terminal(attack);
        assert!(!safe.contains('\x1b'));
        assert_eq!(safe, "innocent?]52;c;cHduZWQ=?\\.txt");
    }

    #[test]
    fn strips_cr_output_forgery() {
        assert_eq!(sanitize_for_terminal("A\rFAKE OUTPUT"), "A?FAKE OUTPUT");
    }

    #[test]
    fn strips_osc8_hyperlink_injection() {
        let attack = "phish\x1b]8;;https:evil.example\x1b\\phony.txt";
        assert!(!sanitize_for_terminal(attack).contains('\x1b'));
    }

    #[test]
    fn strips_del() {
        assert_eq!(sanitize_for_terminal("a\x7fb"), "a?b");
    }

    #[test]
    fn strips_bel_and_null() {
        assert_eq!(sanitize_for_terminal("a\x07b"), "a?b");
        assert_eq!(sanitize_for_terminal("a\0b"), "a?b");
    }

    #[test]
    fn strips_newline() {
        assert_eq!(sanitize_for_terminal("a\nb"), "a?b");
    }

    #[test]
    fn strips_c1_csi_and_osc_initiators() {
        // U+009B is CSI, U+009D is OSC; dangerous on 8-bit-control terminals.
        assert_eq!(sanitize_for_terminal("\u{9b}31m"), "?31m");
        assert_eq!(sanitize_for_terminal("\u{9d}0;pwned\u{9c}"), "?0;pwned?");
    }
}

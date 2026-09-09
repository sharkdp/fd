use crate::sanitize::sanitize_for_terminal_except_newline;

pub fn print_error(msg: impl std::fmt::Display) {
    eprintln!("{}", format_error(&msg.to_string()));
}

/// Build the `[fd error]: ...` line, escaping terminal control characters in
/// `msg` while preserving newlines so multi-line errors stay readable.
fn format_error(msg: &str) -> String {
    format!("[fd error]: {}", sanitize_for_terminal_except_newline(msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_error_escapes_control_chars_but_keeps_newlines() {
        let msg = format_error("path\x1b]0;pwned\x07.txt\nsecond line");
        assert_eq!(msg, "[fd error]: path\\x1B]0;pwned\\x07.txt\nsecond line");
    }

    #[test]
    fn format_error_passes_plain_text_through() {
        let msg = format_error("Search path 'fake' is not a directory.");
        assert_eq!(msg, "[fd error]: Search path 'fake' is not a directory.");
    }
}

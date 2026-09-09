use std::io::IsTerminal;

use crate::sanitize::sanitize_message;

pub fn print_error(msg: impl std::fmt::Display) {
    let msg = msg.to_string();
    // Messages can contain found paths, so escape them like regular output on a terminal.
    if std::io::stderr().is_terminal() {
        eprintln!("[fd error]: {}", sanitize_message(&msg));
    } else {
        eprintln!("[fd error]: {msg}");
    }
}

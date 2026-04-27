use std::io::IsTerminal;

use crate::sanitize::sanitize_for_terminal;

pub fn print_error(msg: impl Into<String>) {
    let msg = msg.into();
    if std::io::stderr().is_terminal() {
        eprintln!("[fd error]: {}", sanitize_for_terminal(&msg));
    } else {
        eprintln!("[fd error]: {msg}");
    }
}

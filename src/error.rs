use std::io::IsTerminal;

use crate::sanitize::maybe_sanitize;

pub fn print_error(msg: impl Into<String>) {
    let msg = msg.into();
    let safe = maybe_sanitize(&msg, std::io::stderr().is_terminal());
    eprintln!("[fd error]: {safe}");
}

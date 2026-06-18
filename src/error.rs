pub fn print_error(msg: impl Into<String>) {
    eprintln!("[fd error]: {}", msg.into());
}

pub fn print_status(msg: impl Into<String>) {
    eprintln!("[fd]: {}", msg.into());
}

pub fn print_error(msg: impl Into<String>) {
    eprintln!("[fd error]: {}", msg.into());
}

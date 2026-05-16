pub fn print_error(msg: impl Into<String>) {
    eprintln!("[fd error]: {}", msg.into());
}

pub fn print_warning(msg: impl Into<String>) {
    eprintln!("[fd warning]: {}", msg.into());
}

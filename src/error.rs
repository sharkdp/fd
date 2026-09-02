pub fn print_error(msg: impl std::fmt::Display) {
    eprintln!("[fd error]: {msg}");
}

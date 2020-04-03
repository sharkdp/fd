macro_rules! print_error {
    ($($arg:tt)*) => (eprintln!("[fd error]: {}", format!($($arg)*)))
}

macro_rules! print_error_and_exit {
    ($($arg:tt)*) => {
        print_error!($($arg)*);
        ::std::process::exit(1);
    };
}

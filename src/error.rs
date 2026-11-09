use std::fmt::Display;
use std::io;

use crate::sanitize::write_sanitized;

/// Print an error message using a format string, sanitizing any arguments if necessary
macro_rules! print_error {
    ($fmt:literal, $($arg:expr),*) => {
        eprintln!(concat!("[fd error]: ", $fmt), $($crate::error::SanitizeErr::sanitize($arg)),*)
    }
}

/// Trait for specifying how to sanitize a type for error display, if needed.
pub(crate) trait SanitizeErr {
    type Sanitized: Display;

    fn sanitize(self) -> Self::Sanitized;
}

impl SanitizeErr for &str {
    type Sanitized = Self;

    fn sanitize(self) -> Self {
        self
    }
}

impl SanitizeErr for io::Error {
    type Sanitized = SanitizedError<io::Error>;

    fn sanitize(self) -> Self::Sanitized {
        SanitizedError(self)
    }
}

impl SanitizeErr for ignore::Error {
    type Sanitized = SanitizedError<ignore::Error>;

    fn sanitize(self) -> Self::Sanitized {
        SanitizedError(self)
    }
}

pub struct SanitizedError<E>(E);

impl<E: std::error::Error> Display for SanitizedError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = self.0.to_string();
        write_sanitized(f, &raw)
    }
}

use std::fmt::Display;
use std::io;

use crate::sanitize::write_sanitized_preserving_newlines;

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
        write_sanitized_preserving_newlines(f, &raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_error_controls_without_escaping_newlines() {
        let error = io::Error::new(
            io::ErrorKind::Other,
            "path\x1b]0;pwned\x07.txt\nsecond line",
        );

        assert_eq!(
            SanitizedError(error).to_string(),
            "path\\x1B]0;pwned\\x07.txt\nsecond line"
        );
    }
}

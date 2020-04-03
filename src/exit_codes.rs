#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExitCode {
    Success,
    GeneralError,
    KilledBySigint,
}

impl Into<i32> for ExitCode {
    fn into(self) -> i32 {
        match self {
            ExitCode::Success => 0,
            ExitCode::GeneralError => 1,
            ExitCode::KilledBySigint => 130,
        }
    }
}

impl ExitCode {
    fn is_error(&self) -> bool {
        *self != ExitCode::Success
    }
}

pub fn merge_exitcodes(results: &[ExitCode]) -> ExitCode {
    if results.iter().any(ExitCode::is_error) {
        return ExitCode::GeneralError;
    }
    ExitCode::Success
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_when_no_results() {
        assert_eq!(merge_exitcodes(&[]), ExitCode::Success);
    }

    #[test]
    fn general_error_if_at_least_one_error() {
        assert_eq!(
            merge_exitcodes(&[ExitCode::GeneralError]),
            ExitCode::GeneralError
        );
        assert_eq!(
            merge_exitcodes(&[ExitCode::KilledBySigint]),
            ExitCode::GeneralError
        );
        assert_eq!(
            merge_exitcodes(&[ExitCode::KilledBySigint, ExitCode::Success]),
            ExitCode::GeneralError
        );
        assert_eq!(
            merge_exitcodes(&[ExitCode::Success, ExitCode::GeneralError]),
            ExitCode::GeneralError
        );
        assert_eq!(
            merge_exitcodes(&[ExitCode::GeneralError, ExitCode::KilledBySigint]),
            ExitCode::GeneralError
        );
    }

    #[test]
    fn success_if_no_error() {
        assert_eq!(merge_exitcodes(&[ExitCode::Success]), ExitCode::Success);
        assert_eq!(
            merge_exitcodes(&[ExitCode::Success, ExitCode::Success]),
            ExitCode::Success
        );
    }
}

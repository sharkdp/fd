#[derive(PartialEq, Debug)]
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
        match self {
            ExitCode::GeneralError | ExitCode::KilledBySigint => true,
            _ => false,
        }
    }
}

pub fn merge_exitcodes(results: Vec<ExitCode>) -> ExitCode {
    if results.iter().any(ExitCode::is_error) {
        return ExitCode::GeneralError;
    }
    ExitCode::Success
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_with_empty_vec() {
        assert_eq!(merge_exitcodes(vec![]), ExitCode::Success);
    }

    #[test]
    fn general_error_with_at_least_a_matching_error() {
        assert_eq!(
            merge_exitcodes(vec![ExitCode::KilledBySigint, ExitCode::Success]),
            ExitCode::GeneralError
        );
        assert_eq!(
            merge_exitcodes(vec![ExitCode::GeneralError, ExitCode::Success]),
            ExitCode::GeneralError
        );
    }

    #[test]
    fn success_with_no_error() {
        assert_eq!(merge_exitcodes(vec![ExitCode::Success]), ExitCode::Success);
    }
}

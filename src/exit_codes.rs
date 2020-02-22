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

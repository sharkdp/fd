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
    pub fn error_if_any_error(results: Vec<Self>) -> Self {
        if results.iter().any(|s| match s {
            ExitCode::GeneralError => true,
            _ => false,
        }) {
            return ExitCode::GeneralError;
        }
        ExitCode::Success
    }
}

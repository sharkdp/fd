pub enum ExitCode {
    Success,
    GeneralError,
    KilledBySigint,
}

impl Into<i32> for ExitCode {
    fn into(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::GeneralError => 1,
            Self::KilledBySigint => 130,
        }
    }
}

impl ExitCode {
    pub fn error_if_any_error(results: Vec<Self>) -> Self {
        if results.iter().any(|s| match s {
            Self::GeneralError => true,
            _ => false,
        }) {
            return Self::GeneralError;
        }
        Self::Success
    }
}

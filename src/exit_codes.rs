pub enum ExitCode {
    GeneralError,
    KilledBySigint,
}

impl Into<i32> for ExitCode {
    fn into(self) -> i32 {
        match self {
            ExitCode::GeneralError => 1,
            ExitCode::KilledBySigint => 130,
        }
    }
}

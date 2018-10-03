pub enum ExitCode {
    Error,
    Sigint,
}

impl Into<i32> for ExitCode {
    fn into(self) -> i32 {
        match self {
            ExitCode::Error => 1,
            ExitCode::Sigint => 130,
        }
    }
}

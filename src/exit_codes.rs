use std::process;

#[cfg(unix)]
use nix::sys::signal::{SigHandler, Signal, raise, signal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success,
    GeneralError,
    KilledBySigint,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> Self {
        match code {
            ExitCode::Success => 0,
            ExitCode::GeneralError => 1,
            ExitCode::KilledBySigint => 130,
        }
    }
}

impl ExitCode {
    /// Exit the process with the appropriate code.
    pub fn exit(self) -> ! {
        #[cfg(unix)]
        if self == ExitCode::KilledBySigint {
            // Get rid of the SIGINT handler, if present, and raise SIGINT
            unsafe {
                if signal(Signal::SIGINT, SigHandler::SigDfl).is_ok() {
                    let _ = raise(Signal::SIGINT);
                }
            }
        }

        process::exit(self.into())
    }

    /// Merge another exit code with the current exit code.
    ///
    /// Currently returns GeneralError if either is an error.
    /// In the future may merge more intelligently.
    pub fn merge(self, other: ExitCode) -> ExitCode {
        if self == ExitCode::Success && other == ExitCode::Success {
            ExitCode::Success
        } else {
            ExitCode::GeneralError
        }
    }

    /// Create an exit code based on the number of results.
    ///
    /// `n_results` is the number of results.
    pub fn has_results(n_results: usize) -> ExitCode {
        if n_results > 0 {
            ExitCode::Success
        } else {
            ExitCode::GeneralError
        }
    }
}

pub fn merge_exitcodes(results: impl IntoIterator<Item = ExitCode>) -> ExitCode {
    results.into_iter().fold(ExitCode::Success, ExitCode::merge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_when_no_results() {
        assert_eq!(merge_exitcodes([]), ExitCode::Success);
    }

    #[test]
    fn general_error_if_at_least_one_error() {
        assert_eq!(
            merge_exitcodes([ExitCode::GeneralError]),
            ExitCode::GeneralError
        );
        assert_eq!(
            merge_exitcodes([ExitCode::KilledBySigint]),
            ExitCode::GeneralError
        );
        assert_eq!(
            merge_exitcodes([ExitCode::KilledBySigint, ExitCode::Success]),
            ExitCode::GeneralError
        );
        assert_eq!(
            merge_exitcodes([ExitCode::Success, ExitCode::GeneralError]),
            ExitCode::GeneralError
        );
        assert_eq!(
            merge_exitcodes([ExitCode::GeneralError, ExitCode::KilledBySigint]),
            ExitCode::GeneralError
        );
    }

    #[test]
    fn success_if_no_error() {
        assert_eq!(merge_exitcodes([ExitCode::Success]), ExitCode::Success);
        assert_eq!(
            merge_exitcodes([ExitCode::Success, ExitCode::Success]),
            ExitCode::Success
        );
    }
}

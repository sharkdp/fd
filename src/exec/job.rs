use crate::config::Config;
use crate::error::print_error;
use crate::exit_codes::{ExitCode, merge_exitcodes};
use crate::walk::WorkerResult;

use super::CommandSet;

/// An event loop that listens for inputs from the `rx` receiver. Each received input will
/// generate a command with the supplied command template. The generated command will then
/// be executed, and this process will continue until the receiver's sender has closed.
pub fn job(
    results: impl IntoIterator<Item = WorkerResult>,
    cmd: &CommandSet,
    config: &Config,
) -> ExitCode {
    // Output should be buffered when only running a single thread
    let buffer_output: bool = config.threads > 1;

    let mut ret = ExitCode::Success;
    for result in results {
        // Obtain the next result from the receiver, else if the channel
        // has closed, exit from the loop
        let dir_entry = match result {
            WorkerResult::Entry(dir_entry) => dir_entry,
            WorkerResult::Error(err) => {
                if config.show_filesystem_errors {
                    print_error(err.to_string());
                }
                continue;
            }
            WorkerResult::FatalError(err) => {
                if config.show_filesystem_errors {
                    print_error(err.to_string());
                }
                return ExitCode::GeneralError;
            }
        };

        // Generate a command, execute it and store its exit code.
        let code = cmd.execute(
            dir_entry.stripped_path(config),
            config.path_separator.as_deref(),
            config.null_separator,
            buffer_output,
        );
        ret = merge_exitcodes([ret, code]);
    }
    // Returns error in case of any error.
    ret
}

pub fn batch(
    results: impl IntoIterator<Item = WorkerResult>,
    cmd: &CommandSet,
    config: &Config,
) -> ExitCode {
    let mut paths = Vec::new();

    for worker_result in results {
        match worker_result {
            WorkerResult::Entry(dir_entry) => paths.push(dir_entry.into_stripped_path(config)),
            WorkerResult::Error(err) => {
                if config.show_filesystem_errors {
                    print_error(err.to_string());
                }
            }
            WorkerResult::FatalError(err) => {
                if config.show_filesystem_errors {
                    print_error(err.to_string());
                }
                return ExitCode::GeneralError;
            }
        }
    }

    cmd.execute_batch(
        paths.into_iter(),
        config.batch_size,
        config.path_separator.as_deref(),
    )
}

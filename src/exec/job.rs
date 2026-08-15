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
                // A traversal error means the search was incomplete; reflect that in
                // the exit code even if it isn't printed (i.e. without --show-errors).
                ret = merge_exitcodes([ret, ExitCode::GeneralError]);
                continue;
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
    // Tracks whether any `WorkerResult::Error` was seen while draining `paths` below, so
    // that a traversal error can still affect the exit code even though it isn't a path
    // and therefore can't be passed through to `execute_batch`.
    let had_filesystem_error = std::cell::Cell::new(false);

    let paths = results
        .into_iter()
        .filter_map(|worker_result| match worker_result {
            WorkerResult::Entry(dir_entry) => Some(dir_entry.into_stripped_path(config)),
            WorkerResult::Error(err) => {
                if config.show_filesystem_errors {
                    print_error(err.to_string());
                }
                had_filesystem_error.set(true);
                None
            }
        });

    let exit_code = cmd.execute_batch(paths, config.batch_size, config.path_separator.as_deref());

    if had_filesystem_error.get() {
        merge_exitcodes([exit_code, ExitCode::GeneralError])
    } else {
        exit_code
    }
}

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use flume::Receiver;

use crate::error::print_error;
use crate::exit_codes::{merge_exitcodes, ExitCode};
use crate::walk::WorkerResult;

use super::CommandTemplate;

/// An event loop that listens for inputs from the `rx` receiver. Each received input will
/// generate a command with the supplied command template. The generated command will then
/// be executed, and this process will continue until the receiver's sender has closed.
pub fn job(
    rx: Arc<Mutex<Receiver<WorkerResult>>>,
    cmd: Arc<CommandTemplate>,
    out_perm: Arc<Mutex<()>>,
    show_filesystem_errors: bool,
    buffer_output: bool,
) -> ExitCode {
    let mut results: Vec<ExitCode> = Vec::new();
    loop {
        // Create a lock on the shared receiver for this thread.
        let lock = rx.lock().unwrap();

        // Obtain the next result from the receiver, else if the channel
        // has closed, exit from the loop
        let value: PathBuf = match lock.recv() {
            Ok(WorkerResult::Entry(path)) => path,
            Ok(WorkerResult::Error(err)) => {
                if show_filesystem_errors {
                    print_error(err.to_string());
                }
                continue;
            }
            Err(_) => break,
        };

        // Drop the lock so that other threads can read from the receiver.
        drop(lock);
        // Generate a command, execute it and store its exit code.
        results.push(cmd.generate_and_execute(&value, Arc::clone(&out_perm), buffer_output))
    }
    // Returns error in case of any error.
    merge_exitcodes(results)
}

pub fn batch(
    rx: Receiver<WorkerResult>,
    cmd: &CommandTemplate,
    show_filesystem_errors: bool,
    buffer_output: bool,
    limit: usize,
) -> ExitCode {
    let paths = rx.iter().filter_map(|value| match value {
        WorkerResult::Entry(path) => Some(path),
        WorkerResult::Error(err) => {
            if show_filesystem_errors {
                print_error(err.to_string());
            }
            None
        }
    });
    if limit == 0 {
        // no limit
        return cmd.generate_and_execute_batch(paths, buffer_output);
    }

    let mut exit_codes = Vec::new();
    let mut peekable = paths.peekable();
    while peekable.peek().is_some() {
        let limited = peekable.by_ref().take(limit);
        let exit_code = cmd.generate_and_execute_batch(limited, buffer_output);
        exit_codes.push(exit_code);
    }
    merge_exitcodes(exit_codes)
}

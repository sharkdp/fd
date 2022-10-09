use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::dir_entry::DirEntry;
use crate::error::print_error;
use crate::exit_codes::{merge_exitcodes, ExitCode};
use crate::walk::WorkerResult;

use super::CommandSet;

/// An event loop that listens for inputs from the `rx` receiver. Each received input will
/// generate a command with the supplied command template. The generated command will then
/// be executed, and this process will continue until the receiver's sender has closed.
pub fn job(
    rx: Arc<Mutex<Receiver<WorkerResult>>>,
    cmd: Arc<CommandSet>,
    out_perm: Arc<Mutex<()>>,
    config: &Config,
) -> ExitCode {
    // Output should be buffered when only running a single thread
    let buffer_output: bool = config.threads > 1;

    let mut results: Vec<ExitCode> = Vec::new();
    loop {
        // Create a lock on the shared receiver for this thread.
        let lock = rx.lock().unwrap();

        // Obtain the next result from the receiver, else if the channel
        // has closed, exit from the loop
        let dir_entry: DirEntry = match lock.recv() {
            Ok(WorkerResult::Entry(dir_entry)) => dir_entry,
            Ok(WorkerResult::Error(err)) => {
                if config.show_filesystem_errors {
                    print_error(err.to_string());
                }
                continue;
            }
            Err(_) => break,
        };

        // Drop the lock so that other threads can read from the receiver.
        drop(lock);
        // Generate a command, execute it and store its exit code.
        results.push(cmd.execute(
            dir_entry.stripped_path(config),
            config.path_separator.as_deref(),
            Arc::clone(&out_perm),
            buffer_output,
        ))
    }
    // Returns error in case of any error.
    merge_exitcodes(results)
}

pub fn batch(rx: Receiver<WorkerResult>, cmd: &CommandSet, config: &Config) -> ExitCode {
    let paths = rx
        .into_iter()
        .filter_map(|worker_result| match worker_result {
            WorkerResult::Entry(dir_entry) => Some(dir_entry.into_stripped_path(config)),
            WorkerResult::Error(err) => {
                if config.show_filesystem_errors {
                    print_error(err.to_string());
                }
                None
            }
        });

    cmd.execute_batch(paths, config.batch_size, config.path_separator.as_deref())
}

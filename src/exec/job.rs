// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use super::CommandTemplate;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use walk::WorkerResult;

/// An event loop that listens for inputs from the `rx` receiver. Each received input will
/// generate a command with the supplied command template. The generated command will then
/// be executed, and this process will continue until the receiver's sender has closed.
pub fn job(
    rx: Arc<Mutex<Receiver<WorkerResult>>>,
    cmd: Arc<CommandTemplate>,
    out_perm: Arc<Mutex<()>>,
    show_filesystem_errors: bool,
) {
    loop {
        // Create a lock on the shared receiver for this thread.
        let lock = rx.lock().unwrap();

        // Obtain the next result from the receiver, else if the channel
        // has closed, exit from the loop
        let value: PathBuf = match lock.recv() {
            Ok(WorkerResult::Entry(val)) => val,
            Ok(WorkerResult::Error(err)) => {
                if show_filesystem_errors {
                    print_error!("{}", err);
                }
                continue;
            }
            Err(_) => break,
        };

        // Drop the lock so that other threads can read from the the receiver.
        drop(lock);
        // Generate a command and execute it.
        cmd.generate_and_execute(&value, Arc::clone(&out_perm));
    }
}

// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use super::CommandTemplate;
use crate::exit_codes::ExitCode;
use crate::walk::WorkerResult;
use crossbeam_channel::Receiver;
use std::sync::{Arc, Mutex};

fn read_values(
    rx: Receiver<WorkerResult>,
    show_filesystem_errors: bool,
) -> impl Iterator<Item = std::path::PathBuf> {
    rx.into_iter().filter_map(move |value| match value {
        WorkerResult::Entry(val) => Some(val),
        WorkerResult::Error(err) => {
            if show_filesystem_errors {
                print_error!("{}", err);
            }
            None
        }
    })
}

/// An event loop that listens for inputs from the `rx` receiver. Each received input will
/// generate a command with the supplied command template. The generated command will then
/// be executed, and this process will continue until the receiver's sender has closed.
pub fn job(
    rx: Receiver<WorkerResult>,
    cmd: Arc<CommandTemplate>,
    out_perm: Arc<Mutex<()>>,
    show_filesystem_errors: bool,
) {
    read_values(rx, show_filesystem_errors)
        .for_each(|value| cmd.generate_and_execute(&value, Arc::clone(&out_perm)));
}

pub fn batch(
    rx: Receiver<WorkerResult>,
    cmd: &CommandTemplate,
    show_filesystem_errors: bool,
) -> ExitCode {
    cmd.generate_and_execute_batch(read_values(rx, show_filesystem_errors))
}

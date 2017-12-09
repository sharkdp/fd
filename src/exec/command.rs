// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::io;
use std::io::Write;

/// Executes a command.
pub fn execute_command(mut cmd: Command, out_perm: Arc<Mutex<()>>) {
    // Spawn the supplied command.
    let output = cmd.output();

    // Then wait for the command to exit, if it was spawned.
    match output {
        Ok(output) => {
            // While this lock is active, this thread will be the only thread allowed
            // to write its outputs.
            let _lock = out_perm.lock().unwrap();

            let stdout = io::stdout();
            let stderr = io::stderr();

            let _ = stdout.lock().write_all(&output.stdout);
            let _ = stderr.lock().write_all(&output.stderr);
        }
        Err(why) => {
            if why.kind() == io::ErrorKind::NotFound {
                eprintln!("fd: execution error: command not found");
            } else {
                eprintln!("fd: execution error: {}", why);
            }
        }
    }
}

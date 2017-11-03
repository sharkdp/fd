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

/// A state that offers access to executing a generated command.
///
/// The ticket holds the collection of arguments of a command to be executed.
pub struct CommandTicket {
    args: Vec<String>,
    out_perm: Arc<Mutex<()>>,
}

impl CommandTicket {
    pub fn new<I, S>(args: I, out_perm: Arc<Mutex<()>>) -> CommandTicket
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        CommandTicket {
            args: args.into_iter().map(|x| x.as_ref().to_owned()).collect(),
            out_perm: out_perm,
        }
    }

    /// Executes the command stored within the ticket.
    #[cfg(not(unix))]
    pub fn then_execute(self) {
        use std::process::Stdio;
        use std::io::Write;

        // Spawn the supplied command.
        let cmd = Command::new(&self.args[0])
            .args(&self.args[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        // Then wait for the command to exit, if it was spawned.
        match cmd {
            Ok(output) => {
                // While this lock is active, this thread will be the only thread allowed
                // to write it's outputs.
                let _lock = self.out_perm.lock().unwrap();

                let stdout = io::stdout();
                let stderr = io::stderr();

                let _ = stdout.lock().write_all(&output.stdout);
                let _ = stderr.lock().write_all(&output.stderr);
            }
            Err(why) => eprintln!("fd: exec error: {}", why),
        }
    }

    #[cfg(all(unix))]
    pub fn then_execute(self) {
        use libc::{close, dup2, pipe, STDERR_FILENO, STDOUT_FILENO};
        use std::fs::File;
        use std::os::unix::process::CommandExt;
        use std::os::unix::io::FromRawFd;

        // Initial a pair of pipes that will be used to
        // pipe the std{out,err} of the spawned process.
        let mut stdout_fds = [0; 2];
        let mut stderr_fds = [0; 2];

        unsafe {
            pipe(stdout_fds.as_mut_ptr());
            pipe(stderr_fds.as_mut_ptr());
        }

        // Spawn the supplied command.
        let cmd = Command::new(&self.args[0])
            .args(&self.args[1..])
            // Configure the pipes accordingly in the child.
            .before_exec(move || unsafe {
                // Redirect the child's std{out,err} to the write ends of our pipe.
                dup2(stdout_fds[1], STDOUT_FILENO);
                dup2(stderr_fds[1], STDERR_FILENO);

                // Close all the fds we created here, so EOF will be sent when the program exits.
                close(stdout_fds[0]);
                close(stdout_fds[1]);
                close(stderr_fds[0]);
                close(stderr_fds[1]);
                Ok(())
            })
            .spawn();

        // Open the read end of the pipes as `File`s.
        let (mut pout, mut perr) = unsafe {
            // Close the write ends of the pipes in the parent
            close(stdout_fds[1]);
            close(stderr_fds[1]);
            (
                // But create files from the read ends.
                File::from_raw_fd(stdout_fds[0]),
                File::from_raw_fd(stderr_fds[0]),
            )
        };

        match cmd {
            Ok(mut child) => {
                let _ = child.wait();

                // Create a lock to ensure that this thread has exclusive access to writing.
                let _lock = self.out_perm.lock().unwrap();

                // And then write the outputs of the process until EOF is sent to each file.
                let stdout = io::stdout();
                let stderr = io::stderr();
                let _ = io::copy(&mut pout, &mut stdout.lock());
                let _ = io::copy(&mut perr, &mut stderr.lock());
            }
            Err(why) => eprintln!("fd: exec error: {}", why),
        }
    }
}

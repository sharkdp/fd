use std::env;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::io;

lazy_static! {
    /// On non-Windows systems, the `SHELL` environment variable will be used to determine the
    /// preferred shell of choice for execution. Windows will simply use `cmd`.
    static ref COMMAND: (String, &'static str) = if cfg!(target_os = "windows") {
        ("cmd".into(), "/C")
    } else {
        (env::var("SHELL").unwrap_or("/bin/sh".into()), "-c")
    };
}

/// A state that offers access to executing a generated command.
///
/// The ticket holds a mutable reference to a string that contains the command to be executed.
/// After execution of the the command via the `then_execute()` method, the string will be
/// cleared so that a new command can be written to the string in the future.
pub struct CommandTicket<'a> {
    command: &'a mut String,
    out_perm: Arc<Mutex<()>>,
}

impl<'a> CommandTicket<'a> {
    pub fn new(command: &'a mut String, out_perm: Arc<Mutex<()>>) -> CommandTicket<'a> {
        CommandTicket { command, out_perm }
    }

    /// Executes the command stored within the ticket, and
    /// clearing the command's buffer when finished.'
    #[cfg(not(all(unix, not(target_os = "redox"))))]
    pub fn then_execute(self) {
        use std::process::Stdio;
        use std::io::Write;

        // Spawn a shell with the supplied command.
        let cmd = Command::new(COMMAND.0.as_str())
            .arg(COMMAND.1)
            .arg(&self.command)
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

        // Clear the buffer for later re-use.
        self.command.clear();
    }

    #[cfg(all(unix, not(target_os = "redox")))]
    pub fn then_execute(self) {
        use libc::*;
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

        // Spawn a shell with the supplied command.
        let cmd = Command::new(COMMAND.0.as_str())
            .arg(COMMAND.1)
            .arg(&self.command)
            .before_exec(move || unsafe {
                // Configure the pipes accordingly in the child.
                dup2(stdout_fds[1], STDOUT_FILENO);
                dup2(stderr_fds[1], STDERR_FILENO);
                close(stdout_fds[0]);
                close(stdout_fds[1]);
                close(stderr_fds[0]);
                close(stderr_fds[1]);
                Ok(())
            })
            .spawn();

        // Open the read end of the pipes as `File`s.
        let (mut pout, mut perr) = unsafe {
            close(stdout_fds[1]);
            close(stderr_fds[1]);
            (
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

        // Clear the command string's buffer for later re-use.
        self.command.clear();
    }
}

use std::process::Command;
use std::env;

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
}

impl<'a> CommandTicket<'a> {
    pub fn new(command: &'a mut String) -> CommandTicket<'a> {
        CommandTicket { command }
    }

    /// Executes the command stored within the ticket, and
    /// clearing the command's buffer when finished.
    pub fn then_execute(self) {
        // Spawn a shell with the supplied command.
        let cmd = Command::new(COMMAND.0.as_str())
            .arg(COMMAND.1)
            .arg(&self.command)
            .spawn();

        // Then wait for the command to exit, if it was spawned.
        match cmd {
            Ok(mut child) => { let _ = child.wait(); },
            Err(why) => eprintln!("fd: exec error: {}", why),
        }

        // Clear the buffer for later re-use.
        self.command.clear();
    }
}

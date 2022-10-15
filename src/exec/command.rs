use std::io;
use std::io::Write;
use std::sync::Mutex;

use argmax::Command;

use crate::error::print_error;
use crate::exit_codes::ExitCode;

struct Outputs {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
struct OutputBuffer<'a> {
    output_permission: &'a Mutex<()>,
    outputs: Vec<Outputs>,
}

impl<'a> OutputBuffer<'a> {
    fn new(output_permission: &'a Mutex<()>) -> Self {
        Self {
            output_permission,
            outputs: Vec::new(),
        }
    }

    fn push(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        self.outputs.push(Outputs { stdout, stderr });
    }

    fn write(self) {
        // avoid taking the lock if there is nothing to do
        if self.outputs.is_empty() {
            return;
        }
        // While this lock is active, this thread will be the only thread allowed
        // to write its outputs.
        let _lock = self.output_permission.lock().unwrap();

        let stdout = io::stdout();
        let stderr = io::stderr();

        let mut stdout = stdout.lock();
        let mut stderr = stderr.lock();

        for output in self.outputs.iter() {
            let _ = stdout.write_all(&output.stdout);
            let _ = stderr.write_all(&output.stderr);
        }
    }
}

/// Executes a command.
pub fn execute_commands<I: Iterator<Item = io::Result<Command>>>(
    cmds: I,
    out_perm: &Mutex<()>,
    enable_output_buffering: bool,
) -> ExitCode {
    let mut output_buffer = OutputBuffer::new(out_perm);
    for result in cmds {
        let mut cmd = match result {
            Ok(cmd) => cmd,
            Err(e) => {
                print_cmd_error(e);
                return ExitCode::GeneralError;
            }
        };

        // Spawn the supplied command.
        let output = if enable_output_buffering {
            cmd.output()
        } else {
            // If running on only one thread, don't buffer output
            // Allows for viewing and interacting with intermediate command output
            cmd.spawn().and_then(|c| c.wait_with_output())
        };

        // Then wait for the command to exit, if it was spawned.
        match output {
            Ok(output) => {
                if enable_output_buffering {
                    output_buffer.push(output.stdout, output.stderr);
                }
                if output.status.code() != Some(0) {
                    output_buffer.write();
                    return ExitCode::GeneralError;
                }
            }
            Err(why) => {
                output_buffer.write();
                return handle_cmd_error(&cmd, CommandError::Io(why));
            }
        }
    }
    output_buffer.write();
    ExitCode::Success
}

pub enum CommandError {
    Io(io::Error),
    Failed,
}

impl From<io::Error> for CommandError {
    fn from(e: io::Error) -> Self {
        CommandError::Io(e)
    }
}

pub fn handle_cmd_error(cmd: &Command, err: CommandError) -> ExitCode {
    match (cmd, err) {
        (_, CommandError::Failed) => {
            // The child process probably already wrote an error message if appropriate
        }
        (cmd, CommandError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
            print_error(format!(
                "Command not found: {}",
                cmd.get_program().to_string_lossy()
            ));
        }
        (_, CommandError::Io(err)) => print_cmd_error(err),
    };
    ExitCode::GeneralError
}

pub fn print_cmd_error(err: io::Error) {
    print_error(format!("Problem while executing command: {}", err));
}

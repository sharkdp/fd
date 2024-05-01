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
            Err(e) => return handle_cmd_error(None, e),
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
                return handle_cmd_error(Some(&cmd), why);
            }
        }
    }
    output_buffer.write();
    ExitCode::Success
}

pub fn execute_commands_filtering<I: Iterator<Item = io::Result<Command>>>(
    cmds: I,
    out_perm: &Mutex<()>,
    enable_output_buffering: bool,
) -> ExitCode {
    ExitCode::Success
}

pub fn handle_cmd_error(cmd: Option<&Command>, err: io::Error) -> ExitCode {
    match (cmd, err) {
        (Some(cmd), err) if err.kind() == io::ErrorKind::NotFound => {
            print_error(format!(
                "Command not found: {}",
                cmd.get_program().to_string_lossy()
            ));
            ExitCode::GeneralError
        }
        (_, err) => {
            print_error(format!("Problem while executing command: {}", err));
            ExitCode::GeneralError
        }
    }
}

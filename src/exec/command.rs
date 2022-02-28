use std::io;
use std::io::Write;
use std::process::Command;
use std::sync::Mutex;

use crate::error::print_error;
use crate::exit_codes::ExitCode;

struct Outputs {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
struct OutputBuf<'a> {
    out_perm: &'a Mutex<()>,
    outputs: Vec<Outputs>,
}

impl<'a> OutputBuf<'a> {
    fn new(out_perm: &'a Mutex<()>) -> Self {
        Self {
            out_perm,
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
        let _lock = self.out_perm.lock().unwrap();

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
pub fn execute_commands<I: Iterator<Item = Command>>(
    cmds: I,
    out_perm: &Mutex<()>,
    enable_output_buffering: bool,
) -> ExitCode {
    let mut out_buf = OutputBuf::new(out_perm);
    for mut cmd in cmds {
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
                    out_buf.push(output.stdout, output.stderr);
                }
                if output.status.code() != Some(0) {
                    out_buf.write();
                    return ExitCode::GeneralError;
                }
            }
            Err(why) => {
                out_buf.write();
                return handle_cmd_error(&cmd, why);
            }
        }
    }
    out_buf.write();
    ExitCode::Success
}

pub fn handle_cmd_error(cmd: &Command, err: io::Error) -> ExitCode {
    if err.kind() == io::ErrorKind::NotFound {
        print_error(format!("Command not found: {:?}", cmd));
        ExitCode::GeneralError
    } else {
        print_error(format!("Problem while executing command: {}", err));
        ExitCode::GeneralError
    }
}

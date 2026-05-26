use std::io;
use std::io::Write;
use std::sync::Mutex;
use std::thread;

use argmax::Command;
use crossbeam_channel::unbounded;

use crate::error::print_error;
use crate::exit_codes::ExitCode;

struct Outputs {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
pub struct OutputBuffer {
    null_separator: bool,
    outputs: Vec<Outputs>,
}

impl OutputBuffer {
    pub fn new(null_separator: bool) -> Self {
        Self {
            null_separator,
            outputs: Vec::new(),
        }
    }

    fn push(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        self.outputs.push(Outputs { stdout, stderr });
    }

    fn write(self) {
        // Avoid taking the lock if there is nothing to do.
        // If null_separator is true, then we still need to write the
        // null separator, because the output may have been written directly
        // to stdout
        if self.outputs.is_empty() && !self.null_separator {
            return;
        }

        let stdout = io::stdout();
        let stderr = io::stderr();

        // While we hold these locks, only this thread will be able
        // to write its outputs.
        let mut stdout = stdout.lock();
        let mut stderr = stderr.lock();

        for output in self.outputs.iter() {
            let _ = stdout.write_all(&output.stdout);
            let _ = stderr.write_all(&output.stderr);
        }
        if self.null_separator {
            // If null_separator is enabled, then we should write a \0 at the end
            // of the output for this entry
            let _ = stdout.write_all(b"\0");
        }
    }
}

/// Executes a command.
pub fn execute_commands<I: Iterator<Item = io::Result<Command>>>(
    cmds: I,
    mut output_buffer: OutputBuffer,
    enable_output_buffering: bool,
) -> ExitCode {
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

/// Execute a list of batch commands, optionally in parallel.
pub fn execute_batch_commands(commands: Vec<Command>, parallel: bool, threads: usize) -> ExitCode {
    if commands.is_empty() {
        return ExitCode::Success;
    }

    if parallel && threads > 1 {
        execute_batch_commands_parallel(commands, threads)
    } else {
        execute_batch_commands_sequential(commands)
    }
}

fn execute_batch_commands_sequential(mut commands: Vec<Command>) -> ExitCode {
    let mut exit_code = ExitCode::Success;

    for mut cmd in commands.drain(..) {
        match cmd.status() {
            Ok(status) => {
                if !status.success() {
                    exit_code = ExitCode::GeneralError;
                }
            }
            Err(err) => return handle_cmd_error(Some(&cmd), err),
        }
    }

    exit_code
}

fn execute_batch_commands_parallel(commands: Vec<Command>, threads: usize) -> ExitCode {
    let num_workers = threads.min(commands.len());
    let (work_tx, work_rx) = unbounded::<Command>();
    for cmd in commands {
        let _ = work_tx.send(cmd);
    }
    drop(work_tx);

    let (result_tx, result_rx) = unbounded::<BatchCommandResult>();
    let stdout_lock = Mutex::new(());
    let stderr_lock = Mutex::new(());

    thread::scope(|scope| {
        for _ in 0..num_workers {
            let work_rx = work_rx.clone();
            let result_tx = result_tx.clone();

            scope.spawn(move || {
                while let Ok(mut cmd) = work_rx.recv() {
                    let result = match cmd.output() {
                        Ok(output) => BatchCommandResult {
                            stdout: output.stdout,
                            stderr: output.stderr,
                            success: output.status.success(),
                            cmd,
                            err: None,
                        },
                        Err(err) => BatchCommandResult {
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                            success: false,
                            cmd,
                            err: Some(err),
                        },
                    };

                    if result_tx.send(result).is_err() {
                        break;
                    }
                }
            });
        }
    });

    drop(result_tx);

    let mut exit_code = ExitCode::Success;

    for result in result_rx {
        if let Some(err) = result.err {
            return handle_cmd_error(Some(&result.cmd), err);
        }

        {
            let _lock = stdout_lock.lock().unwrap();
            let _ = io::stdout().lock().write_all(&result.stdout);
        }
        {
            let _lock = stderr_lock.lock().unwrap();
            let _ = io::stderr().lock().write_all(&result.stderr);
        }

        if !result.success {
            exit_code = ExitCode::GeneralError;
        }
    }

    exit_code
}

struct BatchCommandResult {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    success: bool,
    cmd: Command,
    err: Option<io::Error>,
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
            print_error(format!("Problem while executing command: {err}"));
            ExitCode::GeneralError
        }
    }
}

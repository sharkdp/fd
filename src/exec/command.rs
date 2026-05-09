use std::io;
use std::io::Write;

use argmax::Command;

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

/// Common shell builtins that typically do not exist as standalone executables.
/// When fd encounters a "command not found" error for one of these, it hints
/// that the user may be trying to use a shell builtin.
const SHELL_BUILTINS: &[&str] = &[
    ".", "alias", "bg", "bind", "cd", "command", "declare", "dirs", "eval", "exec", "exit",
    "export", "fg", "hash", "help", "history", "jobs", "let", "local", "logout", "popd", "pushd",
    "read", "readonly", "return", "set", "shift", "shopt", "source", "suspend", "times", "trap",
    "type", "typeset", "unalias", "unset", "wait",
];

fn is_shell_builtin(program: &str) -> bool {
    SHELL_BUILTINS.iter().any(|&b| b == program)
}

fn command_not_found_message(program: &str) -> String {
    if is_shell_builtin(program) {
        format!(
            "Command not found: {program}. Note: {program} is a shell builtin, \
             not a standalone program. To run shell builtins, invoke a shell explicitly, \
             e.g. fd -x sh -c '{program} ... \"$1\"' sh {{}}",
        )
    } else {
        format!("Command not found: {program}")
    }
}

pub fn handle_cmd_error(cmd: Option<&Command>, err: io::Error) -> ExitCode {
    match (cmd, err) {
        (Some(cmd), err) if err.kind() == io::ErrorKind::NotFound => {
            let program = cmd.get_program().to_string_lossy();
            print_error(command_not_found_message(&program));
            ExitCode::GeneralError
        }
        (_, err) => {
            print_error(format!("Problem while executing command: {err}"));
            ExitCode::GeneralError
        }
    }
}

#[cfg(test)]
mod builtin_tests {
    use super::*;

    #[test]
    fn detects_known_builtins() {
        assert!(is_shell_builtin("cd"));
        assert!(is_shell_builtin("export"));
        assert!(is_shell_builtin("source"));
        assert!(is_shell_builtin("eval"));
        assert!(is_shell_builtin("."));
    }

    #[test]
    fn rejects_non_builtins() {
        assert!(!is_shell_builtin("grep"));
        assert!(!is_shell_builtin("ls"));
        assert!(!is_shell_builtin(""));
        assert!(!is_shell_builtin("CD"));
        // These typically exist as standalone executables
        assert!(!is_shell_builtin("echo"));
        assert!(!is_shell_builtin("printf"));
        assert!(!is_shell_builtin("test"));
    }

    #[test]
    fn builtin_message_includes_hint() {
        let msg = command_not_found_message("cd");
        assert!(msg.starts_with("Command not found: cd."));
        assert!(msg.contains("shell builtin"));
        assert!(msg.contains("sh -c"));
    }

    #[test]
    fn non_builtin_message_is_plain() {
        let msg = command_not_found_message("nonexistent");
        assert_eq!(msg, "Command not found: nonexistent");
        assert!(!msg.contains("shell builtin"));
    }
}

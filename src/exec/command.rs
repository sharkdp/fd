use std::io;
use std::io::Write;
use std::process::Command;
use std::sync::Mutex;

use crate::error::print_error;
use crate::exit_codes::ExitCode;

/// Executes a command.
pub fn execute_command(
    mut cmd: Command,
    out_perm: &Mutex<()>,
    enable_output_buffering: bool,
    dry_run: bool,
) -> ExitCode {
    if dry_run {
        let _lock = out_perm.lock().unwrap();
        let stdout = io::stdout();

        let out = format!(
            "{} {}\n",
            cmd.get_program().to_str().unwrap_or_default(),
            cmd.get_args()
                .map(|arg| arg.to_str().unwrap_or_default())
                .collect::<Vec<_>>()
                .join(" ")
        );

        let _ = stdout.lock().write_all(out.as_bytes());

        return ExitCode::Success;
    }

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
            // While this lock is active, this thread will be the only thread allowed
            // to write its outputs.
            let _lock = out_perm.lock().unwrap();

            let stdout = io::stdout();
            let stderr = io::stderr();

            let _ = stdout.lock().write_all(&output.stdout);
            let _ = stderr.lock().write_all(&output.stderr);

            if output.status.code() == Some(0) {
                ExitCode::Success
            } else {
                ExitCode::GeneralError
            }
        }
        Err(ref why) if why.kind() == io::ErrorKind::NotFound => {
            print_error(format!("Command not found: {:?}", cmd));
            ExitCode::GeneralError
        }
        Err(why) => {
            print_error(format!("Problem while executing command: {}", why));
            ExitCode::GeneralError
        }
    }
}

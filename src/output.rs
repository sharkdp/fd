use std::io::{self, StdoutLock, Write};
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use lscolors::{LsColors, Style};

use crate::config::Config;
use crate::error::print_error;
use crate::exit_codes::ExitCode;
use crate::filesystem::strip_current_dir;

fn replace_path_separator(path: &str, new_path_separator: &str) -> String {
    path.replace(std::path::MAIN_SEPARATOR, new_path_separator)
}

// TODO: this function is performance critical and can probably be optimized
pub fn print_entry(
    stdout: &mut StdoutLock,
    entry: &Path,
    config: &Config,
    wants_to_quit: &Arc<AtomicBool>,
) {
    let path = if config.strip_cwd_prefix {
        strip_current_dir(entry)
    } else {
        entry
    };

    let r = if let Some(ref ls_colors) = config.ls_colors {
        print_entry_colorized(stdout, path, config, ls_colors, wants_to_quit)
    } else {
        print_entry_uncolorized(stdout, path, config)
    };

    if let Err(e) = r {
        if e.kind() == ::std::io::ErrorKind::BrokenPipe {
            // Exit gracefully in case of a broken pipe (e.g. 'fd ... | head -n 3').
            process::exit(0);
        } else {
            print_error(format!("Could not write to output: {}", e));
            process::exit(ExitCode::GeneralError.into());
        }
    }
}

// TODO: this function is performance critical and can probably be optimized
fn print_entry_colorized(
    stdout: &mut StdoutLock,
    path: &Path,
    config: &Config,
    ls_colors: &LsColors,
    wants_to_quit: &Arc<AtomicBool>,
) -> io::Result<()> {
    let default_style = ansi_term::Style::default();

    // Traverse the path and colorize each component
    for (component, style) in ls_colors.style_for_path_components(path) {
        let style = style
            .map(Style::to_ansi_term_style)
            .unwrap_or(default_style);

        let mut path_string = component.to_string_lossy();
        if let Some(ref separator) = config.path_separator {
            *path_string.to_mut() = replace_path_separator(&path_string, separator);
        }
        write!(stdout, "{}", style.paint(path_string))?;

        // TODO: can we move this out of the if-statement? Why do we call it that often?
        if wants_to_quit.load(Ordering::Relaxed) {
            writeln!(stdout)?;
            process::exit(ExitCode::KilledBySigint.into());
        }
    }

    if config.null_separator {
        write!(stdout, "\0")
    } else {
        writeln!(stdout)
    }
}

// TODO: this function is performance critical and can probably be optimized
fn print_entry_uncolorized_base(
    stdout: &mut StdoutLock,
    path: &Path,
    config: &Config,
) -> io::Result<()> {
    let separator = if config.null_separator { "\0" } else { "\n" };

    let mut path_string = path.to_string_lossy();
    if let Some(ref separator) = config.path_separator {
        *path_string.to_mut() = replace_path_separator(&path_string, separator);
    }
    write!(stdout, "{}{}", path_string, separator)
}

#[cfg(not(unix))]
fn print_entry_uncolorized(
    stdout: &mut StdoutLock,
    path: &Path,
    config: &Config,
) -> io::Result<()> {
    print_entry_uncolorized_base(stdout, path, config)
}

#[cfg(unix)]
fn print_entry_uncolorized(
    stdout: &mut StdoutLock,
    path: &Path,
    config: &Config,
) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    if config.interactive_terminal || config.path_separator.is_some() {
        // Fall back to the base implementation
        print_entry_uncolorized_base(stdout, path, config)
    } else {
        // Print path as raw bytes, allowing invalid UTF-8 filenames to be passed to other processes
        let separator = if config.null_separator { b"\0" } else { b"\n" };
        stdout.write_all(path.as_os_str().as_bytes())?;
        stdout.write_all(separator)
    }
}

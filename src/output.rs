use std::borrow::Cow;
use std::io::{self, Write};
use std::path::Path;

use lscolors::{Indicator, LsColors, Style};

use crate::config::Config;
use crate::error::print_error;
use crate::exit_codes::ExitCode;
use crate::filesystem::strip_current_dir;

fn replace_path_separator(path: &str, new_path_separator: &str) -> String {
    path.replace(std::path::MAIN_SEPARATOR, new_path_separator)
}

// TODO: this function is performance critical and can probably be optimized
pub fn print_entry<W: Write>(stdout: &mut W, entry: &Path, config: &Config) {
    let path = if config.strip_cwd_prefix {
        strip_current_dir(entry)
    } else {
        entry
    };

    let r = if let Some(ref ls_colors) = config.ls_colors {
        print_entry_colorized(stdout, path, config, ls_colors)
    } else {
        print_entry_uncolorized(stdout, path, config)
    };

    if let Err(e) = r {
        if e.kind() == ::std::io::ErrorKind::BrokenPipe {
            // Exit gracefully in case of a broken pipe (e.g. 'fd ... | head -n 3').
            ExitCode::Success.exit();
        } else {
            print_error(format!("Could not write to output: {}", e));
            ExitCode::GeneralError.exit();
        }
    }
}

// TODO: this function is performance critical and can probably be optimized
fn print_entry_colorized<W: Write>(
    stdout: &mut W,
    path: &Path,
    config: &Config,
    ls_colors: &LsColors,
) -> io::Result<()> {
    // Split the path between the parent and the last component
    let mut offset = 0;
    let path_str = path.to_string_lossy();

    if let Some(parent) = path.parent() {
        offset = parent.to_string_lossy().len();
        for c in path_str[offset..].chars() {
            if std::path::is_separator(c) {
                offset += c.len_utf8();
            } else {
                break;
            }
        }
    }

    if offset > 0 {
        let mut parent_str = Cow::from(&path_str[..offset]);
        if let Some(ref separator) = config.path_separator {
            *parent_str.to_mut() = replace_path_separator(&parent_str, separator);
        }

        let style = ls_colors
            .style_for_indicator(Indicator::Directory)
            .map(Style::to_ansi_term_style)
            .unwrap_or_default();
        write!(stdout, "{}", style.paint(parent_str))?;
    }

    let style = ls_colors
        .style_for_path(path)
        .map(Style::to_ansi_term_style)
        .unwrap_or_default();
    write!(stdout, "{}", style.paint(&path_str[offset..]))?;

    if config.null_separator {
        write!(stdout, "\0")?;
    } else {
        writeln!(stdout)?;
    }

    Ok(())
}

// TODO: this function is performance critical and can probably be optimized
fn print_entry_uncolorized_base<W: Write>(
    stdout: &mut W,
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
fn print_entry_uncolorized<W: Write>(
    stdout: &mut W,
    path: &Path,
    config: &Config,
) -> io::Result<()> {
    print_entry_uncolorized_base(stdout, path, config)
}

#[cfg(unix)]
fn print_entry_uncolorized<W: Write>(
    stdout: &mut W,
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

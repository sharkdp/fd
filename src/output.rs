use std::borrow::Cow;
use std::io::{self, Write};

use lscolors::{Indicator, LsColors, Style};

use crate::config::Config;
use crate::dir_entry::DirEntry;
use crate::error::print_error;
use crate::exit_codes::ExitCode;

fn replace_path_separator(path: &str, new_path_separator: &str) -> String {
    path.replace(std::path::MAIN_SEPARATOR, new_path_separator)
}

// TODO: this function is performance critical and can probably be optimized
pub fn print_entry<W: Write>(stdout: &mut W, entry: &DirEntry, config: &Config) {
    let r = if let Some(ref ls_colors) = config.ls_colors {
        print_entry_colorized(stdout, entry, config, ls_colors)
    } else {
        print_entry_uncolorized(stdout, entry, config)
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

// Display a trailing slash if the path is a directory and the config option is enabled.
// If the path_separator option is set, display that instead.
// The trailing slash will not be colored.
#[inline]
fn print_trailing_slash<W: Write>(
    stdout: &mut W,
    entry: &DirEntry,
    config: &Config,
    style: Option<&Style>,
) -> io::Result<()> {
    if entry.file_type().map_or(false, |ft| ft.is_dir()) {
        write!(
            stdout,
            "{}",
            style
                .map(Style::to_nu_ansi_term_style)
                .unwrap_or_default()
                .paint(&config.actual_path_separator)
        )?;
    }
    Ok(())
}

// Trying to copy: https://www.gnu.org/software/coreutils/quotes.html
fn path_needs_quoting(path: &str) -> i8 {
    // If it contains any special chars we return single quote
    if path.contains(" ") || path.contains("$") || path.contains("\"") {
        return 1;
    // If it ONLY contains a ' we return double quote
    } else if path.contains("'") {
        return 2;
    }

    return 0;
}

// Quote a path with coreutils style quoting to make copy/paste
// more friendly for shells
fn quote_path(path_str: &str) -> String {
    let quote_type         = path_needs_quoting(path_str);
    let mut tmp_str:String = path_str.into();

    // Quote with single quotes
    if quote_type == 1 {
        // Escape single quotes in path
        tmp_str = str::replace(&tmp_str, "'", "'\\''");

        format!("'{}'", tmp_str)
    // Quote with double quotes
    } else if quote_type == 2 {
        // Escape double quotes in path
        tmp_str = str::replace(&tmp_str, "\"", "\\\"");

        format!("\"{}\"", tmp_str)
    // No quoting required
    } else {
        path_str.to_string()
    }
}

// TODO: this function is performance critical and can probably be optimized
fn print_entry_colorized<W: Write>(
    stdout: &mut W,
    entry: &DirEntry,
    config: &Config,
    ls_colors: &LsColors,
) -> io::Result<()> {
    // Split the path between the parent and the last component
    let mut offset        = 0;
    let path              = entry.stripped_path(config);
    let mut path_str      = path.to_string_lossy();
    let mut needs_quoting = false;

    // Wrap the path in quotes
    if config.use_quoting {
        let tmp_str = quote_path(&path_str);

        // If the quoted string is new, then we flag that to tweak the offset
        // so the colors line up
        if tmp_str != path_str {
            path_str      = tmp_str.into();
            needs_quoting = true;
        }
    }

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

        if needs_quoting {
            offset += 2;
        }

        let mut parent_str = Cow::from(&path_str[..offset]);
        if let Some(ref separator) = config.path_separator {
            *parent_str.to_mut() = replace_path_separator(&parent_str, separator);
        }

        let style = ls_colors
            .style_for_indicator(Indicator::Directory)
            .map(Style::to_nu_ansi_term_style)
            .unwrap_or_default();
        write!(stdout, "{}", style.paint(parent_str))?;
    }

    let style = entry
        .style(ls_colors)
        .map(Style::to_nu_ansi_term_style)
        .unwrap_or_default();
    write!(stdout, "{}", style.paint(&path_str[offset..]))?;

    print_trailing_slash(
        stdout,
        entry,
        config,
        ls_colors.style_for_indicator(Indicator::Directory),
    )?;

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
    entry: &DirEntry,
    config: &Config,
) -> io::Result<()> {
    let separator = if config.null_separator { "\0" } else { "\n" };
    let path = entry.stripped_path(config);

    let mut path_string = path.to_string_lossy();
    if let Some(ref separator) = config.path_separator {
        *path_string.to_mut() = replace_path_separator(&path_string, separator);
    }
    write!(stdout, "{}", path_string)?;
    print_trailing_slash(stdout, entry, config, None)?;
    write!(stdout, "{}", separator)
}

#[cfg(not(unix))]
fn print_entry_uncolorized<W: Write>(
    stdout: &mut W,
    entry: &DirEntry,
    config: &Config,
) -> io::Result<()> {
    print_entry_uncolorized_base(stdout, entry, config)
}

#[cfg(unix)]
fn print_entry_uncolorized<W: Write>(
    stdout: &mut W,
    entry: &DirEntry,
    config: &Config,
) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    if config.interactive_terminal || config.path_separator.is_some() {
        // Fall back to the base implementation
        print_entry_uncolorized_base(stdout, entry, config)
    } else {
        // Print path as raw bytes, allowing invalid UTF-8 filenames to be passed to other processes
        let separator = if config.null_separator { b"\0" } else { b"\n" };
        stdout.write_all(entry.stripped_path(config).as_os_str().as_bytes())?;
        print_trailing_slash(stdout, entry, config, None)?;
        stdout.write_all(separator)
    }
}

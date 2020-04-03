use std::borrow::Cow;
use std::io::{self, StdoutLock, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ansi_term;
use lscolors::{LsColors, Style};

use crate::exit_codes::ExitCode;
use crate::filesystem::strip_current_dir;
use crate::options::Options;

pub fn print_entry(
    stdout: &mut StdoutLock,
    entry: &PathBuf,
    config: &Options,
    wants_to_quit: &Arc<AtomicBool>,
) {
    let path = if entry.is_absolute() {
        entry.as_path()
    } else {
        strip_current_dir(entry)
    };

    let r = if let Some(ref ls_colors) = config.ls_colors {
        print_entry_colorized(stdout, path, config, ls_colors, &wants_to_quit)
    } else {
        print_entry_uncolorized(stdout, path, config)
    };

    if r.is_err() {
        // Probably a broken pipe. Exit gracefully.
        process::exit(ExitCode::GeneralError.into());
    }
}

fn replace_path_separator<'a>(config: &Options, path: &mut Cow<'a, str>) {
    match &config.path_separator {
        None => {}
        Some(sep) => {
            *path.to_mut() = path.replace(std::path::MAIN_SEPARATOR, &sep);
        }
    }
}

fn print_entry_colorized(
    stdout: &mut StdoutLock,
    path: &Path,
    config: &Options,
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
        replace_path_separator(&config, &mut path_string);
        write!(stdout, "{}", style.paint(path_string))?;

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

fn print_entry_uncolorized(
    stdout: &mut StdoutLock,
    path: &Path,
    config: &Options,
) -> io::Result<()> {
    let separator = if config.null_separator { "\0" } else { "\n" };

    let mut path_str = path.to_string_lossy();
    replace_path_separator(&config, &mut path_str);
    write!(stdout, "{}{}", path_str, separator)
}

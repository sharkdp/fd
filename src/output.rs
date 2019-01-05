// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use crate::exit_codes::ExitCode;
use crate::internal::opts::FdOptions;
use lscolors::{LsColors, Style};

use std::io::{self, Write};
use std::ops::Deref;
use std::path::{self, Component, Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ansi_term;

/// Remove the `./` prefix from a path.
fn strip_current_dir<'a>(pathbuf: &'a PathBuf) -> &'a Path {
    let mut iter = pathbuf.components();
    let mut iter_next = iter.clone();
    if iter_next.next() == Some(Component::CurDir) {
        iter.next();
    }
    iter.as_path()
}

pub fn print_entry(entry: &PathBuf, config: &FdOptions, wants_to_quit: &Arc<AtomicBool>) {
    let path = if entry.is_absolute() {
        entry.as_path()
    } else {
        strip_current_dir(entry)
    };

    let r = if let Some(ref ls_colors) = config.ls_colors {
        print_entry_colorized(path, config, ls_colors, &wants_to_quit)
    } else {
        print_entry_uncolorized(path, config)
    };

    if r.is_err() {
        // Probably a broken pipe. Exit gracefully.
        process::exit(ExitCode::GeneralError.into());
    }
}

fn print_entry_colorized(
    path: &Path,
    config: &FdOptions,
    ls_colors: &LsColors,
    wants_to_quit: &Arc<AtomicBool>,
) -> io::Result<()> {
    let default_style = ansi_term::Style::default();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Separator to use before the current component.
    let mut separator = String::new();

    // Full path to the current component.
    let mut component_path = PathBuf::new();

    // Traverse the path and colorize each component
    for component in path.components() {
        let comp_str = component.as_os_str().to_string_lossy();
        component_path.push(Path::new(comp_str.deref()));

        let style = ls_colors.style_for_path(&component_path);
        let style = style
            .map(Style::to_ansi_term_style)
            .unwrap_or(default_style);

        write!(handle, "{}{}", separator, style.paint(comp_str))?;

        // Determine separator to print before next component.
        separator = match component {
            // Prefix needs no separator, as it is always followed by RootDir.
            Component::Prefix(_) => String::new(),
            // RootDir is already a separator.
            Component::RootDir => String::new(),
            // Everything else uses a separator that is painted the same way as the component.
            _ => style.paint(path::MAIN_SEPARATOR.to_string()).to_string(),
        };

        if wants_to_quit.load(Ordering::Relaxed) {
            write!(handle, "\n")?;
            process::exit(ExitCode::KilledBySigint.into());
        }
    }

    if config.null_separator {
        write!(handle, "\0")
    } else {
        writeln!(handle, "")
    }
}

fn print_entry_uncolorized(path: &Path, config: &FdOptions) -> io::Result<()> {
    let separator = if config.null_separator { "\0" } else { "\n" };

    let path_str = path.to_string_lossy();
    write!(&mut io::stdout(), "{}{}", path_str, separator)
}

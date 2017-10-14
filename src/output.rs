use internal::{FdOptions, PathDisplay, ROOT_DIR};
use lscolors::LsColors;

use std::{fs, process};
use std::io::{self, Write};
use std::ops::Deref;
use std::path::{self, Path, PathBuf, Component};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use ansi_term;

pub fn print_entry(base: &Path, entry: &PathBuf, config: &FdOptions) {
    let r = if let Some(ref ls_colors) = config.ls_colors {
        print_entry_colorized(base, entry, config, ls_colors)
    } else {
        print_entry_uncolorized(entry, config)
    };

    if r.is_err() {
        // Probably a broken pipe. Exit gracefully.
        process::exit(0);
    }
}

fn print_entry_colorized(
    base: &Path,
    entry: &PathBuf,
    config: &FdOptions,
    ls_colors: &LsColors,
) -> io::Result<()> {
    let default_style = ansi_term::Style::default();

    let mut component_path = base.to_path_buf();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let path_full = base.join(entry);

    if config.path_display == PathDisplay::Absolute {
        write!(handle, "{}", ls_colors.directory.paint(ROOT_DIR))?;
    }

    // Traverse the path and colorize each component
    for component in entry.components() {
        let comp_str = component.as_os_str().to_string_lossy();

        component_path.push(Path::new(comp_str.deref()));

        if let Component::RootDir = component {
            // Printing the root dir would result in too many path separators on Windows.
            // Note that a root dir component won't occur on Unix, because `entry` is never an
            // absolute path in that case.
            continue;
        }

        let metadata = component_path.metadata();
        let is_directory = metadata.as_ref().map(|md| md.is_dir()).unwrap_or(false);

        let style = if component_path
            .symlink_metadata()
            .map(|md| md.file_type().is_symlink())
            .unwrap_or(false)
        {
            &ls_colors.symlink
        } else if is_directory {
            &ls_colors.directory
        } else if metadata.map(|md| is_executable(&md)).unwrap_or(false) {
            &ls_colors.executable
        } else {
            // Look up file name
            let o_style = component_path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| ls_colors.filenames.get(n));

            match o_style {
                        Some(s) => s,
                        None =>
                            // Look up file extension
                            component_path.extension()
                                          .and_then(|e| e.to_str())
                                          .and_then(|e| ls_colors.extensions.get(e))
                                          .unwrap_or(&default_style)
                    }
        };

        write!(handle, "{}", style.paint(comp_str))?;

        if is_directory && component_path != path_full {
            let sep = path::MAIN_SEPARATOR.to_string();
            write!(handle, "{}", style.paint(sep))?;
        }
    }

    if config.null_separator {
        write!(handle, "\0")
    } else {
        writeln!(handle, "")
    }
}

fn print_entry_uncolorized(entry: &PathBuf, config: &FdOptions) -> io::Result<()> {
    // Uncolorized output
    let prefix = if config.path_display == PathDisplay::Absolute {
        ROOT_DIR
    } else {
        ""
    };
    let separator = if config.null_separator { "\0" } else { "\n" };

    let path_str = entry.to_string_lossy();
    write!(&mut io::stdout(), "{}{}{}", prefix, path_str, separator)
}

#[cfg(unix)]
fn is_executable(md: &fs::Metadata) -> bool {
    md.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
fn is_executable(_: &fs::Metadata) -> bool {
    false
}

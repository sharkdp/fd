use internal::{FdOptions, PathDisplay, ROOT_DIR};

use std::{fs, process};
use std::io::{self, Write};
use std::ops::Deref;
use std::path::{self, Path, PathBuf};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use ansi_term;

pub fn print_entry(base: &Path, entry: &PathBuf, config: &FdOptions) {
    let path_full = base.join(entry);

    let path_str = entry.to_string_lossy();

    #[cfg(unix)]
    let is_executable = |p: Option<&fs::Metadata>| {
        p.map(|f| f.permissions().mode() & 0o111 != 0)
         .unwrap_or(false)
    };

    #[cfg(windows)]
    let is_executable = |_: Option<&fs::Metadata>| false;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    if let Some(ref ls_colors) = config.ls_colors {
        let default_style = ansi_term::Style::default();

        let mut component_path = base.to_path_buf();

        if config.path_display == PathDisplay::Absolute {
            print!("{}", ls_colors.directory.paint(ROOT_DIR));
        }

        // Traverse the path and colorize each component
        for component in entry.components() {
            let comp_str = component.as_os_str().to_string_lossy();

            component_path.push(Path::new(comp_str.deref()));

            let metadata = component_path.metadata().ok();
            let is_directory = metadata.as_ref().map(|md| md.is_dir()).unwrap_or(false);

            let style =
                if component_path.symlink_metadata()
                                 .map(|md| md.file_type().is_symlink())
                                 .unwrap_or(false) {
                    &ls_colors.symlink
                } else if is_directory {
                    &ls_colors.directory
                } else if is_executable(metadata.as_ref()) {
                    &ls_colors.executable
                } else {
                    // Look up file name
                    let o_style =
                        component_path.file_name()
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

            write!(handle, "{}", style.paint(comp_str)).ok();

            if is_directory && component_path != path_full {
                let sep = path::MAIN_SEPARATOR.to_string();
                write!(handle, "{}", style.paint(sep)).ok();
            }
        }

        let r = if config.null_separator {
            write!(handle, "\0")
        } else {
            writeln!(handle, "")
        };
        if r.is_err() {
            // Probably a broken pipe. Exit gracefully.
            process::exit(0);
        }
    } else {
        // Uncolorized output

        let prefix = if config.path_display == PathDisplay::Absolute { ROOT_DIR } else { "" };
        let separator = if config.null_separator { "\0" } else { "\n" };

        let r = write!(&mut io::stdout(), "{}{}{}", prefix, path_str, separator);

        if r.is_err() {
            // Probably a broken pipe. Exit gracefully.
            process::exit(0);
        }
    }
}

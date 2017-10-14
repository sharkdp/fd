use internal::{FdOptions, PathDisplay};
use lscolors::LsColors;

use std::{fs, process};
use std::io::{self, Write};
use std::ops::Deref;
use std::path::{self, Path, PathBuf, Component};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use ansi_term;

pub fn print_entry(base: &Path, entry: &PathBuf, config: &FdOptions) {
    let path_full = base.join(entry);

    let path_to_print = if config.path_display == PathDisplay::Absolute {
        &path_full
    } else {
        entry
    };

    let r = if let Some(ref ls_colors) = config.ls_colors {
        print_entry_colorized(base, path_to_print, config, ls_colors)
    } else {
        print_entry_uncolorized(path_to_print, config)
    };

    if r.is_err() {
        // Probably a broken pipe. Exit gracefully.
        process::exit(0);
    }
}

fn print_entry_colorized(
    base: &Path,
    path: &Path,
    config: &FdOptions,
    ls_colors: &LsColors,
) -> io::Result<()> {
    let default_style = ansi_term::Style::default();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Separator to use before the current component.
    let mut separator = String::new();

    // Full path to the current component.
    let mut component_path = base.to_path_buf();

    // Traverse the path and colorize each component
    for component in path.components() {
        let comp_str = component.as_os_str().to_string_lossy();
        component_path.push(Path::new(comp_str.deref()));

        let style = get_path_style(&component_path, ls_colors).unwrap_or(&default_style);

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

fn get_path_style<'a>(path: &Path, ls_colors: &'a LsColors) -> Option<&'a ansi_term::Style> {
    if path.symlink_metadata()
        .map(|md| md.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Some(&ls_colors.symlink);
    }

    let metadata = path.metadata();

    if metadata.as_ref().map(|md| md.is_dir()).unwrap_or(false) {
        Some(&ls_colors.directory)
    } else if metadata.map(|md| is_executable(&md)).unwrap_or(false) {
        Some(&ls_colors.executable)
    } else if let Some(filename_style) =
        path.file_name().and_then(|n| n.to_str()).and_then(|n| {
            ls_colors.filenames.get(n)
        })
    {
        Some(filename_style)
    } else if let Some(extension_style) =
        path.extension().and_then(|e| e.to_str()).and_then(|e| {
            ls_colors.extensions.get(e)
        })
    {
        Some(extension_style)
    } else {
        None
    }
}

#[cfg(unix)]
fn is_executable(md: &fs::Metadata) -> bool {
    md.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
fn is_executable(_: &fs::Metadata) -> bool {
    false
}

use std::borrow::Cow;
use std::io::{self, Write};
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use jiff::{Timestamp, tz::TimeZone};
use lscolors::{Indicator, LsColors, Style};
#[cfg(unix)]
use nix::unistd::{Gid, Group, Uid, User};

use crate::config::Config;
use crate::dir_entry::DirEntry;
use crate::fmt::FormatTemplate;
use crate::hyperlink::PathUrl;

fn replace_path_separator(path: &str, new_path_separator: &str) -> String {
    path.replace(std::path::MAIN_SEPARATOR, new_path_separator)
}

// TODO: this function is performance critical and can probably be optimized
pub fn print_entry<W: Write>(stdout: &mut W, entry: &DirEntry, config: &Config) -> io::Result<()> {
    let mut has_hyperlink = false;
    if config.hyperlink
        && let Some(url) = PathUrl::new(entry.path())
    {
        write!(stdout, "\x1B]8;;{url}\x1B\\")?;
        has_hyperlink = true;
    }

    if config.list_details {
        print_entry_details(stdout, entry, config, &config.ls_colors)?;
    } else if let Some(ref format) = config.format {
        print_entry_format(stdout, entry, config, format)?;
    } else if let Some(ref ls_colors) = config.ls_colors {
        print_entry_colorized(stdout, entry, config, ls_colors)?;
    } else {
        print_entry_uncolorized(stdout, entry, config)?;
    };

    if has_hyperlink {
        write!(stdout, "\x1B]8;;\x1B\\")?;
    }

    if config.null_separator {
        write!(stdout, "\0")
    } else {
        writeln!(stdout)
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
    if entry.file_type().is_some_and(|ft| ft.is_dir()) {
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

// TODO: this function is performance critical and can probably be optimized
fn print_entry_format<W: Write>(
    stdout: &mut W,
    entry: &DirEntry,
    config: &Config,
    format: &FormatTemplate,
) -> io::Result<()> {
    let output = format.generate(
        entry.stripped_path(config),
        config.path_separator.as_deref(),
    );
    // TODO: support writing raw bytes on unix?
    write!(stdout, "{}", output.to_string_lossy())
}

// TODO: this function is performance critical and can probably be optimized
fn print_entry_colorized<W: Write>(
    stdout: &mut W,
    entry: &DirEntry,
    config: &Config,
    ls_colors: &LsColors,
) -> io::Result<()> {
    // Split the path between the parent and the last component
    let mut offset = 0;
    let path = entry.stripped_path(config);
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

    Ok(())
}

// TODO: this function is performance critical and can probably be optimized
fn print_entry_uncolorized_base<W: Write>(
    stdout: &mut W,
    entry: &DirEntry,
    config: &Config,
) -> io::Result<()> {
    let path = entry.stripped_path(config);

    let mut path_string = path.to_string_lossy();
    if let Some(ref separator) = config.path_separator {
        *path_string.to_mut() = replace_path_separator(&path_string, separator);
    }
    write!(stdout, "{path_string}")?;
    print_trailing_slash(stdout, entry, config, None)
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
        stdout.write_all(entry.stripped_path(config).as_os_str().as_bytes())?;
        print_trailing_slash(stdout, entry, config, None)
    }
}

fn format_size(size: u64) -> String {
    if size < 1024 {
        return format!("{} B", size);
    }
    let units = ["K", "M", "G", "T", "P", "E"];
    let mut size = size as f64;
    let mut unit_idx = 0;

    size /= 1024.0;

    while size >= 1024.0 && unit_idx < units.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if size < 10.0 {
        format!("{:.1} {}", size, units[unit_idx])
    } else {
        format!("{:.0} {}", size, units[unit_idx])
    }
}

fn print_entry_details<W: Write>(
    stdout: &mut W,
    entry: &DirEntry,
    config: &Config,
    ls_colors: &Option<LsColors>,
) -> io::Result<()> {
    let metadata = entry.metadata();

    #[cfg(unix)]
    let mode = metadata.map(|m| m.permissions().mode()).unwrap_or(0);
    #[cfg(not(unix))]
    let mode = 0;

    let perms = unix_mode::to_string(mode);

    #[cfg(unix)]
    let nlink = metadata.map(|m| m.nlink()).unwrap_or(1);
    #[cfg(not(unix))]
    let nlink = 1;

    #[cfg(unix)]
    let (user, group) = {
        let uid = metadata.map(|m| m.uid()).unwrap_or(0);
        let gid = metadata.map(|m| m.gid()).unwrap_or(0);
        let user = User::from_uid(Uid::from_raw(uid))
            .ok()
            .flatten()
            .map(|u| u.name)
            .unwrap_or_else(|| uid.to_string());
        let group = Group::from_gid(Gid::from_raw(gid))
            .ok()
            .flatten()
            .map(|g| g.name)
            .unwrap_or_else(|| gid.to_string());
        (user, group)
    };
    #[cfg(not(unix))]
    let (user, group) = ("".to_string(), "".to_string());

    let size = metadata.map(|m| m.len()).unwrap_or(0);
    let size_str = format_size(size);

    let time = metadata
        .and_then(|m| m.modified().ok())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let timestamp = Timestamp::try_from(time).unwrap_or(Timestamp::UNIX_EPOCH);
    let zoned = timestamp.to_zoned(TimeZone::system());
    let date_str = zoned.strftime("%b %d %H:%M").to_string();

    write!(
        stdout,
        "{} {:>3} {:>8} {:>8} {:>8} {} ",
        perms, nlink, user, group, size_str, date_str
    )?;

    if let Some(ls_colors) = ls_colors {
        print_entry_colorized(stdout, entry, config, ls_colors)?;
    } else {
        print_entry_uncolorized(stdout, entry, config)?;
    }

    if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false)
        && let Ok(target) = std::fs::read_link(entry.path())
    {
        write!(stdout, " -> {}", target.to_string_lossy())?;
    }

    Ok(())
}

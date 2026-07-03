use std::borrow::Cow;
use std::io::{self, Write};

use lscolors::{Indicator, LsColors, Style};

use crate::config::Config;
use crate::dir_entry::DirEntry;
use crate::fmt::FormatTemplate;
use crate::hyperlink::PathUrl;
use crate::sanitize::maybe_sanitize;

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

    if let Some(meta) = entry.metadata()
        && config.list_details
    {
        print_details(stdout, meta)?;
    };

    if let Some(ref format) = config.format {
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
    let s = output.to_string_lossy();
    write!(
        stdout,
        "{}",
        maybe_sanitize(&s, config.interactive_terminal)
    )
}

// TODO: this function is performance critical and can probably be optimized
fn print_entry_colorized<W: Write>(
    stdout: &mut W,
    entry: &DirEntry,
    config: &Config,
    ls_colors: &LsColors,
) -> io::Result<()> {
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
        let safe_parent = maybe_sanitize(&parent_str, config.interactive_terminal);
        write!(stdout, "{}", style.paint(safe_parent.as_ref()))?;
    }

    let style = entry
        .style(ls_colors)
        .map(Style::to_nu_ansi_term_style)
        .unwrap_or_default();
    let safe_basename = maybe_sanitize(&path_str[offset..], config.interactive_terminal);
    write!(stdout, "{}", style.paint(safe_basename.as_ref()))?;

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
    let safe = maybe_sanitize(&path_string, config.interactive_terminal);
    write!(stdout, "{safe}")?;
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
        print_entry_uncolorized_base(stdout, entry, config)
    } else {
        // Piped output: raw bytes so invalid UTF-8 filenames reach downstream tools intact.
        stdout.write_all(entry.stripped_path(config).as_os_str().as_bytes())?;
        print_trailing_slash(stdout, entry, config, None)
    }
}

#[cfg(target_os = "windows")]
fn print_details<W: Write>(stdout: &mut W, meta: &std::fs::Metadata) -> io::Result<()> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    const FILE_ATTRIBUTE_ARCHIVE: u32 = 0x20;
    const FILE_ATTRIBUTE_READONLY: u32 = 0x01;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x02;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x04;

    let attrs = meta.file_attributes();
    let mut attr_buf = [b'-'; 6];

    if attrs & FILE_ATTRIBUTE_DIRECTORY != 0 {
        attr_buf[0] = b'd';
    }
    if attrs & FILE_ATTRIBUTE_ARCHIVE != 0 {
        attr_buf[1] = b'a';
    }
    if attrs & FILE_ATTRIBUTE_READONLY != 0 {
        attr_buf[2] = b'r';
    }
    if attrs & FILE_ATTRIBUTE_HIDDEN != 0 {
        attr_buf[3] = b'h';
    }
    if attrs & FILE_ATTRIBUTE_SYSTEM != 0 {
        attr_buf[4] = b's';
    }

    stdout.write_all(&attr_buf)?;
    stdout.write_all(b" ")?;

    print_time(stdout, filetime_to_unix_seconds(meta.last_write_time()))?;

    if meta.is_file() {
        print_size(stdout, meta.len())?;
    } else {
        write!(stdout, "{:>7}", "")?
    }

    stdout.write_all(b"  ")
}

#[cfg(not(target_os = "windows"))]
fn print_details<W: Write>(stdout: &mut W, meta: &std::fs::Metadata) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;

    let mode = meta.mode();
    let mode_string = unix_mode::to_string(mode);

    let nlink = meta.nlink();

    let uid = meta.uid();
    let gid = meta.gid();

    let user_name = users::get_user_by_uid(uid)
        .map(|u| u.name().to_string_lossy().into_owned())
        .unwrap_or_else(|| uid.to_string());

    let group_name = users::get_group_by_gid(gid)
        .map(|g| g.name().to_string_lossy().into_owned())
        .unwrap_or_else(|| gid.to_string());

    write!(
        stdout,
        "{:<10} {:>3} {:<8} {:<8} ",
        mode_string, nlink, user_name, group_name,
    )?;

    print_size(stdout, meta.len())?;
    print_time(stdout, meta.mtime())?;

    stdout.write_all(b" ")
}

fn print_time<W: Write>(stdout: &mut W, unix_seconds: i64) -> io::Result<()> {
    use jiff::{Timestamp, Zoned};

    let timestamp = Timestamp::from_second(unix_seconds)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let zoned: Zoned = timestamp.to_zoned(jiff::tz::TimeZone::system());

    write!(stdout, "  {:>20}", zoned.strftime("%d-%m-%Y %H:%M:%S"))
}

#[cfg(target_os = "windows")]
fn filetime_to_unix_seconds(file_time: u64) -> i64 {
    const INTERVALS_PER_SECOND: u64 = 10_000_000;
    const EPOCH_DIFF_SECONDS: u64 = 11_644_473_600;
    (file_time / INTERVALS_PER_SECOND).saturating_sub(EPOCH_DIFF_SECONDS) as i64
}

fn print_size<W: Write>(stdout: &mut W, size: u64) -> io::Result<()> {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P"];
    let mut sz = size as f64;
    let mut unit_idx = 0;
    while sz >= 1024.0 && unit_idx < UNITS.len() - 1 {
        sz /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        write!(stdout, "{:>6}{}", size, UNITS[unit_idx])
    } else {
        write!(stdout, "{:>6.1}{}", sz, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_size() {
        let re = regex::Regex::new(r"^\s+\d{0,4}(\.\d?)?[BKMGTP]").unwrap();

        let mut buf = Vec::new();

        let mut sz = 1;
        for _ in 0..19 {
            print_size(&mut buf, sz).unwrap();
            let out = String::from_utf8_lossy(&buf);

            assert!(
                re.is_match(&out),
                "{out}\nthe output doesn't math the template"
            );
            sz *= 10;
            buf.clear();
        }

        let mut check = |size: u64, expect: &str| {
            print_size(&mut buf, size).unwrap();
            let out = String::from_utf8_lossy(&buf);
            assert!(
                out.contains(expect),
                "{out}\nthe output doesn't math the template"
            );
            buf.clear();
        };

        check(0, "0B");
        check(1025, "1.0K");
        check(6505537, "6.2M");
    }

    #[test]
    fn test_print_time() {
        let re = regex::Regex::new(r"^\s+\d{2}-\d{2}-\d{4} \d{2}:\d{2}:\d{2}").unwrap();
        let mut buf = Vec::new();

        let mut check = |timestamp| {
            print_time(&mut buf, timestamp).unwrap();
            let out = String::from_utf8_lossy(&buf);
            assert!(
                re.is_match(&out),
                "{out}\nthe output doesn't math the template"
            );
            buf.clear();
        };

        check(0x00000000_00000000);
    }
}

use std::borrow::Cow;
use std::env;
use std::io::{self, Write};

use lscolors::{Indicator, LsColors, Style};

use crate::config::Config;
use crate::dir_entry::DirEntry;
use crate::fmt::FormatTemplate;
use crate::hyperlink::PathUrl;

fn replace_home_dir(path_str: &str) -> Cow<str> {
    if let Ok(home_dir) = env::var("HOME") {
        if path_str == home_dir {
            return Cow::Borrowed("~");
        }
        if path_str.starts_with(&home_dir)
            && path_str.len() > home_dir.len()
            && &path_str[home_dir.len()..home_dir.len() + 1] == std::path::MAIN_SEPARATOR_STR
        {
            let suffix = &path_str[home_dir.len()..];
            let replaced = format!("~{suffix}");
            return Cow::Owned(replaced);
        }
    }
    Cow::Borrowed(path_str)
}

// TODO: this function is performance critical and can probably be optimized
pub fn print_entry<W: Write>(stdout: &mut W, entry: &DirEntry, config: &Config) -> io::Result<()> {
    let mut has_hyperlink = false;
    if config.hyperlink {
        if let Some(url) = PathUrl::new(entry.path()) {
            write!(stdout, "\x1B]8;;{url}\x1B\\")?;
            has_hyperlink = true;
        }
    }

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
    write!(stdout, "{}", output.to_string_lossy())
}

// TODO: this function is performance critical and can probably be optimized
fn print_entry_colorized<W: Write>(
    stdout: &mut W,
    entry: &DirEntry,
    config: &Config,
    ls_colors: &LsColors,
) -> io::Result<()> {
    let path = entry.stripped_path(config);
    let path_str = path.to_string_lossy();

    if let Ok(home_dir) = env::var("HOME") {
        if path == std::path::Path::new(&home_dir) {
            let dir_style = ls_colors
                .style_for_indicator(Indicator::Directory)
                .map(Style::to_nu_ansi_term_style)
                .unwrap_or_default();

            write!(stdout, "{}", dir_style.paint("~"))?;

            print_trailing_slash(
                stdout,
                entry,
                config,
                ls_colors.style_for_indicator(Indicator::Directory),
            )?;
            return Ok(());
        }
    }

    let mut offset = 0;
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
        let parent_part = &path_str[..offset];
        let mut parent_display = replace_home_dir(parent_part);
        if let Some(ref separator) = config.path_separator {
            *parent_display.to_mut() = parent_display.replace(std::path::MAIN_SEPARATOR, separator);
        }
        let dir_style = ls_colors
            .style_for_indicator(Indicator::Directory)
            .map(Style::to_nu_ansi_term_style)
            .unwrap_or_default();
        write!(stdout, "{}", dir_style.paint(&*parent_display))?;
    }

    let file_style = entry
        .style(ls_colors)
        .map(Style::to_nu_ansi_term_style)
        .unwrap_or_default();
    write!(stdout, "{}", file_style.paint(&path_str[offset..]))?;

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
    let path_lossy = path.to_string_lossy();
    let replaced = replace_home_dir(&path_lossy);
    let mut path_string = replaced;
    if let Some(ref separator) = config.path_separator {
        *path_string.to_mut() = path_string.replace(std::path::MAIN_SEPARATOR, separator);
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

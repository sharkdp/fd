use std::borrow::Cow;
use std::io::{self, Write};

use base64::{engine::general_purpose, Engine as _};
use jiff::Timestamp;
use lscolors::{Indicator, LsColors, Style};

use crate::cli::OutputFormat;
use crate::config::Config;
use crate::dir_entry::DirEntry;
use crate::fmt::FormatTemplate;
use crate::hyperlink::PathUrl;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn replace_path_separator(path: &str, new_path_separator: &str) -> String {
    path.replace(std::path::MAIN_SEPARATOR, new_path_separator)
}

#[cfg(unix)]
fn encode_path(path: &std::path::Path) -> PathEncoding {
    use std::os::unix::ffi::OsStrExt;
    let bytes = path.as_os_str().as_bytes();

    // Try to convert to UTF-8 first
    match std::str::from_utf8(bytes) {
        Ok(utf8_str) => {
            let escaped: String = utf8_str.escape_default().collect();
            PathEncoding::Utf8(escaped)
        }
        Err(_) => {
            // Invalid UTF-8, store as raw bytes
            PathEncoding::Bytes(bytes.to_vec())
        }
    }
}

#[cfg(not(unix))]
fn encode_path(path: &std::path::Path) -> PathEncoding {
    // On non-Unix systems, paths are typically UTF-8 or UTF-16
    let path_str = path.to_string_lossy();
    // Always escape the path string for safe output
    // Note: if lossy conversion happened, this might lose information
    let escaped: String = path_str.escape_default().collect();
    PathEncoding::Utf8(escaped)
}

enum PathEncoding {
    Utf8(String),
    Bytes(Vec<u8>),
}

struct FileDetail {
    path: PathEncoding,
    file_type: String,
    size: Option<u64>,
    mode: Option<u32>,
    modified: Option<Timestamp>,
    accessed: Option<Timestamp>,
    created: Option<Timestamp>,
}

pub struct Printer<'a, W> {
    config: &'a Config,
    pub stdout: W,
    started: bool,
}

impl<'a, W: Write> Printer<'a, W> {
    pub fn new(config: &'a Config, stdout: W) -> Self {
        Self {
            config,
            stdout,
            started: false,
        }
    }

    // TODO: this function is performance critical and can probably be optimized
    pub fn print_entry(&mut self, entry: &DirEntry) -> io::Result<()> {
        let mut has_hyperlink = false;
        if self.config.hyperlink {
            if let Some(url) = PathUrl::new(entry.path()) {
                write!(self.stdout, "\x1B]8;;{url}\x1B\\")?;
                has_hyperlink = true;
            }
        }

        match (
            &self.config.format,
            &self.config.output,
            &self.config.ls_colors,
        ) {
            (Some(template), _, _) => self.print_entry_format(entry, template)?,
            (None, OutputFormat::Json, _) => self.print_entry_detail(OutputFormat::Json, entry)?,
            (None, OutputFormat::Yaml, _) => self.print_entry_detail(OutputFormat::Yaml, entry)?,
            (None, OutputFormat::Ndjson, _) => {
                self.print_entry_detail(OutputFormat::Ndjson, entry)?
            }
            (None, OutputFormat::Plain, Some(ls_colors)) => {
                self.print_entry_colorized(entry, ls_colors)?
            }
            (None, OutputFormat::Plain, None) => self.print_entry_uncolorized(entry)?,
        };

        if has_hyperlink {
            write!(self.stdout, "\x1B]8;;\x1B\\")?;
        }

        self.started = true;
        if self.config.null_separator {
            write!(self.stdout, "\0")
        } else if matches!(self.config.output, OutputFormat::Json) {
            Ok(())
        } else {
            writeln!(self.stdout)
        }
    }

    // Display a trailing slash if the path is a directory and the config option is enabled.
    // If the path_separator option is set, display that instead.
    // The trailing slash will not be colored.
    #[inline]
    fn print_trailing_slash(&mut self, entry: &DirEntry, style: Option<&Style>) -> io::Result<()> {
        if entry.file_type().is_some_and(|ft| ft.is_dir()) {
            write!(
                self.stdout,
                "{}",
                style
                    .map(Style::to_nu_ansi_term_style)
                    .unwrap_or_default()
                    .paint(&self.config.actual_path_separator)
            )?;
        }
        Ok(())
    }

    // TODO: this function is performance critical and can probably be optimized
    fn print_entry_format(&mut self, entry: &DirEntry, format: &FormatTemplate) -> io::Result<()> {
        let output = format.generate(
            entry.stripped_path(self.config),
            self.config.path_separator.as_deref(),
        );
        // TODO: support writing raw bytes on unix?
        write!(self.stdout, "{}", output.to_string_lossy())
    }

    // TODO: this function is performance critical and can probably be optimized
    fn print_entry_colorized(&mut self, entry: &DirEntry, ls_colors: &LsColors) -> io::Result<()> {
        // Split the path between the parent and the last component
        let mut offset = 0;
        let path = entry.stripped_path(self.config);
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
            if let Some(ref separator) = self.config.path_separator {
                *parent_str.to_mut() = replace_path_separator(&parent_str, separator);
            }

            let style = ls_colors
                .style_for_indicator(Indicator::Directory)
                .map(Style::to_nu_ansi_term_style)
                .unwrap_or_default();
            write!(self.stdout, "{}", style.paint(parent_str))?;
        }

        let style = entry
            .style(ls_colors)
            .map(Style::to_nu_ansi_term_style)
            .unwrap_or_default();
        write!(self.stdout, "{}", style.paint(&path_str[offset..]))?;

        self.print_trailing_slash(entry, ls_colors.style_for_indicator(Indicator::Directory))?;

        Ok(())
    }

    // TODO: this function is performance critical and can probably be optimized
    fn print_entry_uncolorized_base(&mut self, entry: &DirEntry) -> io::Result<()> {
        let path = entry.stripped_path(self.config);

        let mut path_string = path.to_string_lossy();
        if let Some(ref separator) = self.config.path_separator {
            *path_string.to_mut() = replace_path_separator(&path_string, separator);
        }
        write!(self.stdout, "{path_string}")?;
        self.print_trailing_slash(entry, None)
    }

    #[cfg(not(unix))]
    fn print_entry_uncolorized(&mut self, entry: &DirEntry) -> io::Result<()> {
        self.print_entry_uncolorized_base(entry)
    }

    #[cfg(unix)]
    fn print_entry_uncolorized(&mut self, entry: &DirEntry) -> io::Result<()> {
        use std::os::unix::ffi::OsStrExt;

        if self.config.interactive_terminal || self.config.path_separator.is_some() {
            // Fall back to the base implementation
            self.print_entry_uncolorized_base(entry)
        } else {
            // Print path as raw bytes, allowing invalid UTF-8 filenames to be passed to other processes
            self.stdout
                .write_all(entry.stripped_path(self.config).as_os_str().as_bytes())?;
            self.print_trailing_slash(entry, None)
        }
    }

    fn print_entry_yaml_obj(&mut self, detail: &FileDetail) -> io::Result<()> {
        // Manually construct a simple YAML representation
        // to avoid adding a dependency on serde_yaml (deprecated).
        //
        // Write YAML fragments directly to stdout (should be buffered)
        write!(self.stdout, "- ")?;

        match &detail.path {
            PathEncoding::Utf8(path_utf8) => {
                write!(self.stdout, "path: \"{}\"\n", path_utf8)?;
            }
            PathEncoding::Bytes(path_bytes) => {
                write!(
                    self.stdout,
                    "path_base64: \"{}\"\n",
                    general_purpose::STANDARD.encode(path_bytes)
                )?;
            }
        }

        write!(self.stdout, "  type: {}\n", detail.file_type)?;

        if let Some(size) = detail.size {
            write!(self.stdout, "  size: {size}\n")?;
        }
        if let Some(mode) = detail.mode {
            write!(self.stdout, "  mode: 0o{mode:o}\n")?;
        }
        if let Some(modified) = &detail.modified {
            write!(self.stdout, "  modified: \"{}\"\n", modified)?;
        }
        if let Some(accessed) = &detail.accessed {
            write!(self.stdout, "  accessed: \"{}\"\n", accessed)?;
        }
        if let Some(created) = &detail.created {
            write!(self.stdout, "  created: \"{}\"\n", created)?;
        }
        Ok(())
    }

    fn print_entry_json_obj(&mut self, detail: &FileDetail) -> io::Result<()> {
        if self.started {
            writeln!(self.stdout, ",")?;
        }

        write!(self.stdout, "  {{")?;

        match &detail.path {
            PathEncoding::Utf8(path_utf8) => {
                write!(self.stdout, "\"path\":\"{}\"", path_utf8)?;
            }
            PathEncoding::Bytes(path_bytes) => {
                write!(
                    self.stdout,
                    "\"path_b64\":\"{}\"",
                    general_purpose::STANDARD.encode(path_bytes)
                )?;
            }
        }

        write!(self.stdout, ",\"type\":\"{}\"", detail.file_type)?;

        if let Some(size) = detail.size {
            write!(self.stdout, ",\"size\":{size}")?;
        }
        if let Some(mode) = detail.mode {
            write!(self.stdout, ",\"mode\":{mode:o}")?;
        }
        if let Some(modified) = &detail.modified {
            write!(self.stdout, ",\"modified\":\"{}\"", modified)?;
        }
        if let Some(accessed) = &detail.accessed {
            write!(self.stdout, ",\"accessed\":\"{}\"", accessed)?;
        }
        if let Some(created) = &detail.created {
            write!(self.stdout, ",\"created\":\"{}\"", created)?;
        }
        write!(self.stdout, "}}")
    }

    fn print_entry_detail(&mut self, format: OutputFormat, entry: &DirEntry) -> io::Result<()> {
        let path = entry.stripped_path(self.config);
        let encoded_path = encode_path(path);
        let metadata = entry.metadata();

        let mut detail = FileDetail {
            path: encoded_path,
            file_type: "unknown".to_string(),
            size: None,
            mode: None,
            modified: None,
            accessed: None,
            created: None,
        };
        if let Some(meta) = metadata {
            let size = meta.len();
            let mode = {
                #[cfg(unix)]
                {
                    Some(meta.permissions().mode() & 0o7777)
                }
                #[cfg(not(unix))]
                {
                    None
                }
            };
            let ft = match meta.file_type() {
                ft if ft.is_dir() => "directory",
                ft if ft.is_file() => "file",
                ft if ft.is_symlink() => "symlink",
                _ => "unknown",
            }
            .to_string();

            let modified = meta.modified().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .and_then(|d| Timestamp::from_second(d.as_secs() as i64).ok())
            });

            let accessed = meta.accessed().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .and_then(|d| Timestamp::from_second(d.as_secs() as i64).ok())
            });

            let created = meta.created().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .and_then(|d| Timestamp::from_second(d.as_secs() as i64).ok())
            });

            detail.file_type = ft;
            detail.size = Some(size);
            detail.mode = mode;
            detail.modified = modified;
            detail.accessed = accessed;
            detail.created = created;
        }

        match format {
            OutputFormat::Json => self.print_entry_json_obj(&detail),
            OutputFormat::Yaml => self.print_entry_yaml_obj(&detail),
            OutputFormat::Ndjson => self.print_entry_json_obj(&detail), // NDJSON uses same format as JSON for individual entries
            OutputFormat::Plain => unreachable!("Plain format should not call print_entry_detail"),
        }
    }
}

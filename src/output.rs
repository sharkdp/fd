use std::borrow::Cow;
use std::io::{self, Write};
use std::path::Path;

use base64::{prelude::BASE64_STANDARD, Engine as _};
use jiff::Timestamp;
use lscolors::{Indicator, LsColors, Style};

use crate::cli::OutputFormat;
use crate::config::Config;
use crate::dir_entry::DirEntry;
use crate::exit_codes::ExitCode;
use crate::fmt::FormatTemplate;
use crate::hyperlink::PathUrl;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn replace_path_separator(path: &str, new_path_separator: &str) -> String {
    path.replace(std::path::MAIN_SEPARATOR, new_path_separator)
}

fn encode_path(path: &Path) -> PathEncoding<'_> {
    match path.to_str() {
        Some(utf8) => PathEncoding::Utf8(utf8.escape_default()),
        None => PathEncoding::Bytes(path.as_os_str().as_encoded_bytes()),
    }
}

enum PathEncoding<'a> {
    Utf8(std::str::EscapeDefault<'a>),
    Bytes(&'a [u8]),
}

struct FileDetail<'a> {
    path: PathEncoding<'a>,
    file_type: &'static str,
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

    /// Begin JSON array output if in JSON format.
    /// Returns an error if writing to output fails.
    pub fn begin(&mut self) -> Result<(), ExitCode> {
        if self.config.output == OutputFormat::Json {
            if let Err(e) = writeln!(self.stdout, "[") {
                if e.kind() != ::std::io::ErrorKind::BrokenPipe {
                    crate::error::print_error(format!("Could not write to output: {e}"));
                    return Err(ExitCode::GeneralError);
                }
            }
        }
        Ok(())
    }

    /// End JSON array output if in JSON format.
    /// Returns an error if writing to output fails.
    pub fn end(&mut self) -> Result<(), ExitCode> {
        if self.config.output == OutputFormat::Json {
            if let Err(e) = writeln!(self.stdout, "\n]") {
                if e.kind() != ::std::io::ErrorKind::BrokenPipe {
                    crate::error::print_error(format!("Could not write to output: {e}"));
                    return Err(ExitCode::GeneralError);
                }
            }
        }
        Ok(())
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
            &self.config.jsonl,
            &self.config.ls_colors,
        ) {
            (Some(template), _, _, _) => self.print_entry_format(entry, template)?,
            (None, _, true, _) => self.print_entry_detail(OutputFormat::Jsonl, entry)?,
            (None, OutputFormat::Json, false, _) => {
                self.print_entry_detail(OutputFormat::Json, entry)?
            }
            (None, OutputFormat::Yaml, false, _) => {
                self.print_entry_detail(OutputFormat::Yaml, entry)?
            }
            (None, OutputFormat::Jsonl, false, _) => {
                self.print_entry_detail(OutputFormat::Jsonl, entry)?
            }
            (None, OutputFormat::Plain, false, Some(ls_colors)) => {
                self.print_entry_colorized(entry, ls_colors)?
            }
            (None, OutputFormat::Plain, false, None) => self.print_entry_uncolorized(entry)?,
        };

        if has_hyperlink {
            write!(self.stdout, "\x1B]8;;\x1B\\")?;
        }

        self.started = true;

        if matches!(self.config.output, OutputFormat::Json) {
            return Ok(());
        }

        if self.config.null_separator {
            write!(self.stdout, "\0")
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
            PathEncoding::Utf8(path) => {
                writeln!(self.stdout, "path: \"{}\"", path)?;
            }
            PathEncoding::Bytes(bytes) => {
                writeln!(
                    self.stdout,
                    "path_base64: \"{}\"",
                    BASE64_STANDARD.encode(bytes)
                )?;
            }
        }

        writeln!(self.stdout, "  type: {}", detail.file_type)?;

        if let Some(size) = detail.size {
            writeln!(self.stdout, "  size: {}", size)?;
        }
        if let Some(mode) = detail.mode {
            writeln!(self.stdout, "  mode: 0o{mode:o}")?;
        }
        if let Some(modified) = &detail.modified {
            writeln!(self.stdout, "  modified: \"{}\"", modified)?;
        }
        if let Some(accessed) = &detail.accessed {
            writeln!(self.stdout, "  accessed: \"{}\"", accessed)?;
        }
        if let Some(created) = &detail.created {
            writeln!(self.stdout, "  created: \"{}\"", created)?;
        }
        Ok(())
    }

    fn print_entry_json_obj(&mut self, detail: &FileDetail, comma: bool) -> io::Result<()> {
        if self.started && comma {
            writeln!(self.stdout, ",")?;
        }

        if comma {
            write!(self.stdout, "  ")?;
        }
        write!(self.stdout, "{{")?;

        match &detail.path {
            PathEncoding::Utf8(path_utf8) => {
                write!(self.stdout, "\"path\":\"{}\"", path_utf8)?;
            }
            PathEncoding::Bytes(path_bytes) => {
                write!(
                    self.stdout,
                    "\"path_b64\":\"{}\"",
                    BASE64_STANDARD.encode(path_bytes)
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

        let detail = if let Some(meta) = metadata {
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
            };

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

            FileDetail {
                path: encoded_path,
                file_type: ft,
                size: Some(size),
                mode,
                modified,
                accessed,
                created,
            }
        } else {
            FileDetail {
                path: encoded_path,
                file_type: "unknown",
                size: None,
                mode: None,
                modified: None,
                accessed: None,
                created: None,
            }
        };
        match format {
            OutputFormat::Json => self.print_entry_json_obj(&detail, true),
            OutputFormat::Jsonl => self.print_entry_json_obj(&detail, false),
            OutputFormat::Yaml => self.print_entry_yaml_obj(&detail),
            OutputFormat::Plain => unreachable!("Plain format should not call print_entry_detail"),
        }
    }
}

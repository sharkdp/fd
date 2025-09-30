use std::borrow::Cow;
use std::io::{self, Write};

use lscolors::{Indicator, LsColors, Style};

use crate::config::Config;
use crate::dir_entry::DirEntry;
use crate::fmt::FormatTemplate;
use crate::hyperlink::PathUrl;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

enum DetailFormat {
    Json,
    Yaml,
}

fn replace_path_separator(path: &str, new_path_separator: &str) -> String {
    path.replace(std::path::MAIN_SEPARATOR, new_path_separator)
}

struct FileDetail {
    path: String,
    file_type: String,
    size: Option<u64>,
    mode: Option<u32>,
    modified: Option<u64>,
    accessed: Option<u64>,
    created: Option<u64>,
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

        if let Some(ref format) = self.config.format {
            self.print_entry_format(entry, format)?;
        } else if self.config.yaml {
            self.print_entry_detail(DetailFormat::Yaml, entry)?;
        } else if self.config.json {
            self.print_entry_detail(DetailFormat::Json, entry)?;
        } else if let Some(ref ls_colors) = self.config.ls_colors {
            self.print_entry_colorized(entry, ls_colors)?;
        } else {
            self.print_entry_uncolorized(entry)?;
        };

        if has_hyperlink {
            write!(self.stdout, "\x1B]8;;\x1B\\")?;
        }

        self.started = true;
        if self.config.null_separator {
            write!(self.stdout, "\0")
        } else if self.config.json {
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
    fn print_entry_uncolorized(&self, entry: &DirEntry) -> io::Result<()> {
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
        // A little bit dirty, but safe enough for buffered output.
        let mut result = format!(
            "- path: \"{}\"\n  type: {}\n",
            detail.path, detail.file_type
        );

        if let Some(size) = detail.size {
            result.push_str(&format!("  size: {size}\n"));
        }
        if let Some(mode) = detail.mode {
            result.push_str(&format!("  mode: {mode:o}\n"));
        }
        if let Some(modified) = detail.modified {
            result.push_str(&format!("  modified: {modified}\n"));
        }
        if let Some(accessed) = detail.accessed {
            result.push_str(&format!("  accessed: {accessed}\n"));
        }
        if let Some(created) = detail.created {
            result.push_str(&format!("  created: {created}\n"));
        }
        write!(self.stdout, "{result}")
    }

    fn print_entry_json_obj(&mut self, detail: &FileDetail) -> io::Result<()> {
        // Manually construct a simple JSON representation.
        // A little bit dirty, but safe enough for buffered output.
        let mut result = format!(
            "  {{\"path\":\"{}\",\"type\":\"{}\"",
            detail.path, detail.file_type
        );

        if let Some(size) = detail.size {
            result.push_str(&format!(",\"size\":{size}"));
        }
        if let Some(mode) = detail.mode {
            result.push_str(&format!(",\"mode\":{mode:o}"));
        }
        if let Some(modified) = detail.modified {
            result.push_str(&format!(",\"modified\":{modified}"));
        }
        if let Some(accessed) = detail.accessed {
            result.push_str(&format!(",\"accessed\":{accessed}"));
        }
        if let Some(created) = detail.created {
            result.push_str(&format!(",\"created\":{created}"));
        }
        result.push('}');
        if self.started {
            writeln!(self.stdout, ",")?;
        }
        write!(self.stdout, "{result}")
    }

    fn print_entry_detail(&mut self, format: DetailFormat, entry: &DirEntry) -> io::Result<()> {
        let path = entry.stripped_path(self.config);
        let path_string = path.to_string_lossy().escape_default().to_string();
        let file_type = entry
            .file_type()
            .map(|ft| {
                if ft.is_dir() {
                    "directory"
                } else if ft.is_file() {
                    "file"
                } else if ft.is_symlink() {
                    "symlink"
                } else {
                    "other"
                }
            })
            .unwrap_or("unknown")
            .to_string();
        let metadata = entry.metadata();
        let mut detail = FileDetail {
            path: path_string,
            file_type,
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
            let modified = meta
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs());

            let accessed = meta
                .accessed()?
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs());

            let created = meta
                .created()?
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs());

            detail.size = Some(size);
            detail.mode = mode;
            detail.modified = modified.ok();
            detail.accessed = accessed.ok();
            detail.created = created.ok();
        }

        match format {
            DetailFormat::Json => self.print_entry_json_obj(&detail),
            DetailFormat::Yaml => self.print_entry_yaml_obj(&detail),
        }
    }
}

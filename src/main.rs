#[macro_use]
extern crate clap;
extern crate ansi_term;
extern crate atty;
extern crate regex;
extern crate regex_syntax;
extern crate ignore;
extern crate num_cpus;

pub mod lscolors;
pub mod fshelper;
mod app;
mod internal;
mod output;
mod walk;

use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time;

use atty::Stream;
use regex::RegexBuilder;

use internal::{error, pattern_has_uppercase_char, FdOptions, PathDisplay, ROOT_DIR};
use lscolors::LsColors;
use walk::FileType;

fn main() {
    let matches = app::build_app().get_matches();

    // Get the search pattern
    let empty_pattern = String::new();
    let pattern = matches.value_of("pattern").unwrap_or(&empty_pattern);

    // Get the current working directory
    let current_dir = Path::new(".");
    // .is_dir() is not guarandteed to be intuitively correct for "." and ".."
    if let Err(_) = current_dir.canonicalize() {
        error("Error: could not get current directory.");
    }

    // Get the root directory for the search
    let mut root_dir_buf = match matches.value_of("path") {
        Some(path) => PathBuf::from(path),
        None => current_dir.to_path_buf(),
    };
    if let Err(_) = root_dir_buf.canonicalize() {
        error(&format!(
            "Error: '{}' is not a directory.",
            root_dir_buf.to_string_lossy()
        ));
    }

    let path_display = if matches.is_present("absolute-path") || root_dir_buf.is_absolute() {
        PathDisplay::Absolute
    } else {
        PathDisplay::Relative
    };

    if path_display == PathDisplay::Absolute && root_dir_buf.is_relative() {
        root_dir_buf = fshelper::absolute_path(root_dir_buf.as_path()).unwrap();
    }
    let root_dir = root_dir_buf.as_path();

    // The search will be case-sensitive if the command line flag is set or
    // if the pattern has an uppercase character (smart case).
    let case_sensitive = !matches.is_present("ignore-case") &&
        (matches.is_present("case-sensitive") || pattern_has_uppercase_char(pattern));

    let colored_output = match matches.value_of("color") {
        Some("always") => true,
        Some("never") => false,
        _ => atty::is(Stream::Stdout),
    };

    let ls_colors = if colored_output {
        Some(
            env::var("LS_COLORS")
                .ok()
                .map(|val| LsColors::from_string(&val))
                .unwrap_or_default(),
        )
    } else {
        None
    };

    let config = FdOptions {
        case_sensitive: case_sensitive,
        search_full_path: matches.is_present("full-path"),
        ignore_hidden: !(matches.is_present("hidden") ||
                             matches.occurrences_of("rg-alias-hidden-ignore") >= 2),
        read_ignore: !(matches.is_present("no-ignore") ||
                           matches.is_present("rg-alias-hidden-ignore")),
        follow_links: matches.is_present("follow"),
        null_separator: matches.is_present("null_separator"),
        max_depth: matches.value_of("depth").and_then(|n| {
            usize::from_str_radix(n, 10).ok()
        }),
        threads: std::cmp::max(
            matches
                .value_of("threads")
                .and_then(|n| usize::from_str_radix(n, 10).ok())
                .unwrap_or_else(num_cpus::get),
            1,
        ),
        max_buffer_time: matches
            .value_of("max-buffer-time")
            .and_then(|n| u64::from_str_radix(n, 10).ok())
            .map(time::Duration::from_millis),
        path_display: path_display,
        ls_colors: ls_colors,
        file_type: match matches.value_of("file-type") {
            Some("f") | Some("file") => FileType::RegularFile,
            Some("d") |
            Some("directory") => FileType::Directory,
            Some("l") | Some("symlink") => FileType::SymLink,
            _ => FileType::Any,
        },
        extension: matches.value_of("extension").map(|e| {
            e.trim_left_matches('.').to_lowercase()
        }),
    };

    // If base_dir is ROOT_DIR, then root_dir must be absolute.
    // Otherwise root_dir/entry cannot be turned into an existing relative path from base_dir.
    //
    // We utilize ROOT_DIR to avoid resolving the components of root_dir.
    let base_dir_buf = match config.path_display {
        PathDisplay::Relative => current_dir.to_path_buf(),
        PathDisplay::Absolute => PathBuf::from(ROOT_DIR),
    };
    let base_dir = base_dir_buf.as_path();

    match RegexBuilder::new(pattern)
        .case_insensitive(!config.case_sensitive)
        .dot_matches_new_line(true)
        .build() {
        Ok(re) => walk::scan(root_dir, Arc::new(re), base_dir, Arc::new(config)),
        Err(err) => error(err.description()),
    }
}

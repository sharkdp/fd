// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

extern crate ansi_term;
extern crate atty;
#[macro_use]
extern crate clap;
extern crate ignore;
#[macro_use]
extern crate lazy_static;
#[cfg(all(unix, not(target_os = "redox")))]
extern crate libc;
extern crate num_cpus;
extern crate regex;
extern crate regex_syntax;

pub mod fshelper;
pub mod lscolors;
mod app;
mod exec;
mod internal;
mod output;
mod walk;

#[cfg(windows)]
mod windows;

use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time;

use atty::Stream;
use regex::{RegexBuilder, RegexSetBuilder};

use exec::CommandTemplate;
use internal::{error, pattern_has_uppercase_char, transform_args_with_exec, FdOptions};
use lscolors::LsColors;
use walk::FileType;

fn main() {
    let checked_args = transform_args_with_exec(env::args_os());
    let matches = app::build_app().get_matches_from(checked_args);

    // Get the search pattern
    let pattern = matches.value_of("pattern").unwrap_or("");

    // Get the current working directory
    let current_dir = Path::new(".");
    if !fshelper::is_dir(current_dir) {
        error("Error: could not get current directory.");
    }

    //Get one or more root directories to search.
    let mut dir_vec: Vec<_> = match matches.values_of("path") {
        Some(paths) => paths
            .map(|path| {
                let path_buffer = PathBuf::from(path);
                if !fshelper::is_dir(&path_buffer) {
                    error(&format!(
                        "Error: '{}' is not a directory.",
                        path_buffer.to_string_lossy()
                    ));
                }
                path_buffer
            })
            .collect::<Vec<_>>(),
        None => vec![current_dir.to_path_buf()],
    };

    if matches.is_present("absolute-path") {
        dir_vec = dir_vec
            .iter()
            .map(|path_buffer| fshelper::absolute_path(path_buffer).unwrap())
            .collect();
    }

    // Treat pattern as literal string if '--fixed-strings' is used
    let pattern_regex = if matches.is_present("fixed-strings") {
        regex::escape(pattern)
    } else {
        String::from(pattern)
    };

    // The search will be case-sensitive if the command line flag is set or
    // if the pattern has an uppercase character (smart case).
    let case_sensitive = !matches.is_present("ignore-case")
        && (matches.is_present("case-sensitive") || pattern_has_uppercase_char(&pattern_regex));

    let colored_output = match matches.value_of("color") {
        Some("always") => true,
        Some("never") => false,
        _ => atty::is(Stream::Stdout),
    };

    #[cfg(windows)]
    let colored_output = colored_output && windows::enable_colored_output();

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

    let command = matches.values_of("exec").map(CommandTemplate::new);

    let config = FdOptions {
        case_sensitive,
        search_full_path: matches.is_present("full-path"),
        ignore_hidden: !(matches.is_present("hidden")
            || matches.occurrences_of("rg-alias-hidden-ignore") >= 2),
        read_fdignore: !(matches.is_present("no-ignore")
            || matches.is_present("rg-alias-hidden-ignore")),
        read_vcsignore: !(matches.is_present("no-ignore")
            || matches.is_present("rg-alias-hidden-ignore")
            || matches.is_present("no-ignore-vcs")),
        follow_links: matches.is_present("follow"),
        null_separator: matches.is_present("null_separator"),
        max_depth: matches
            .value_of("depth")
            .and_then(|n| usize::from_str_radix(n, 10).ok()),
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
        ls_colors,
        file_types: match matches.values_of("file-type") {
            None => vec![
                FileType::RegularFile,
                FileType::Directory,
                FileType::SymLink,
            ].into_iter()
                .collect(),
            Some(values) => values
                .map(|value| match value {
                    "f" | "file" => FileType::RegularFile,
                    "d" | "directory" => FileType::Directory,
                    "l" | "symlink" => FileType::SymLink,
                    _ => FileType::RegularFile,
                })
                .collect(),
        },
        extensions: matches.values_of("extension").map(|exts| {
            let patterns = exts.map(|e| e.trim_left_matches('.'))
                .map(|e| format!(r".\.{}$", regex::escape(e)));
            match RegexSetBuilder::new(patterns)
                .case_insensitive(true)
                .build()
            {
                Ok(re) => re,
                Err(err) => error(err.description()),
            }
        }),
        command,
        exclude_patterns: matches
            .values_of("exclude")
            .map(|v| v.map(|p| String::from("!") + p).collect())
            .unwrap_or_else(|| vec![]),
    };

    match RegexBuilder::new(&pattern_regex)
        .case_insensitive(!config.case_sensitive)
        .dot_matches_new_line(true)
        .build()
    {
        Ok(re) => walk::scan(&dir_vec, Arc::new(re), Arc::new(config)),
        Err(err) => error(err.description()),
    }
}

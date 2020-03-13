// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

#[macro_use]
mod internal;

mod app;
mod exec;
mod exit_codes;
pub mod fshelper;
mod output;
mod walk;

use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::time;

use atty::Stream;
use globset::Glob;
use lscolors::LsColors;
use regex::bytes::{RegexBuilder, RegexSetBuilder};

use crate::exec::CommandTemplate;
use crate::internal::{
    filter::{SizeFilter, TimeFilter},
    opts::FdOptions,
    pattern_has_uppercase_char, transform_args_with_exec, FileTypes,
};

// We use jemalloc for performance reasons, see https://github.com/sharkdp/fd/pull/481
#[cfg(all(not(windows), not(target_env = "musl")))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() {
    let checked_args = transform_args_with_exec(env::args_os());
    let matches = app::build_app().get_matches_from(checked_args);

    // Set the current working directory of the process
    if let Some(base_directory) = matches.value_of("base-directory") {
        let basedir = Path::new(base_directory);
        if !fshelper::is_dir(basedir) {
            print_error_and_exit!(
                "The '--base-directory' path ('{}') is not a directory.",
                basedir.to_string_lossy()
            );
        }
        if let Err(e) = env::set_current_dir(basedir) {
            print_error_and_exit!(
                "Could not set '{}' as the current working directory: {}",
                basedir.to_string_lossy(),
                e
            );
        }
    }

    // Get the search pattern
    let pattern = matches.value_of("pattern").unwrap_or("");

    // Get the current working directory
    let current_dir = Path::new(".");
    if !fshelper::is_dir(current_dir) {
        print_error_and_exit!("Could not get current directory.");
    }

    // Get one or more root directories to search.
    let mut dir_vec: Vec<_> = match matches
        .values_of("path")
        .or_else(|| matches.values_of("search-path"))
    {
        Some(paths) => paths
            .map(|path| {
                let path_buffer = PathBuf::from(path);
                if !fshelper::is_dir(&path_buffer) {
                    print_error_and_exit!(
                        "'{}' is not a directory.",
                        path_buffer.to_string_lossy()
                    );
                }
                path_buffer
            })
            .collect::<Vec<_>>(),
        None => vec![current_dir.to_path_buf()],
    };

    if matches.is_present("absolute-path") {
        dir_vec = dir_vec
            .iter()
            .map(|path_buffer| {
                path_buffer
                    .canonicalize()
                    .and_then(|pb| fshelper::absolute_path(pb.as_path()))
                    .unwrap()
            })
            .collect();
    }

    // Detect if the user accidentally supplied a path instead of a search pattern
    if !matches.is_present("full-path")
        && pattern.contains(std::path::MAIN_SEPARATOR)
        && fshelper::is_dir(Path::new(pattern))
    {
        print_error_and_exit!(
            "The search pattern '{pattern}' contains a path-separation character ('{sep}') \
             and will not lead to any search results.\n\n\
             If you want to search for all files inside the '{pattern}' directory, use a match-all pattern:\n\n  \
             fd . '{pattern}'\n\n\
             Instead, if you want to search for the pattern in the full path, use:\n\n  \
             fd --full-path '{pattern}'",
            pattern = pattern,
            sep = std::path::MAIN_SEPARATOR,
        );
    }

    let pattern_regex = if matches.is_present("glob") {
        let glob = match Glob::new(pattern) {
            Ok(glob) => glob,
            Err(e) => {
                print_error_and_exit!("{}", e);
            }
        };
        glob.regex().to_owned()
    } else if matches.is_present("fixed-strings") {
        // Treat pattern as literal string if '--fixed-strings' is used
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

    let path_separator = matches.value_of("path-separator").map(|str| str.to_owned());

    #[cfg(windows)]
    let colored_output = colored_output && ansi_term::enable_ansi_support().is_ok();

    let ls_colors = if colored_output {
        Some(LsColors::from_env().unwrap_or_default())
    } else {
        None
    };

    let command = matches
        .values_of("exec")
        .map(CommandTemplate::new)
        .or_else(|| {
            matches.values_of("exec-batch").map(|m| {
                CommandTemplate::new_batch(m).unwrap_or_else(|e| {
                    print_error_and_exit!("{}", e);
                })
            })
        });

    let size_limits: Vec<SizeFilter> = matches
        .values_of("size")
        .map(|v| {
            v.map(|sf| {
                if let Some(f) = SizeFilter::from_string(sf) {
                    return f;
                }
                print_error_and_exit!("'{}' is not a valid size constraint. See 'fd --help'.", sf);
            })
            .collect()
        })
        .unwrap_or_else(|| vec![]);

    let now = time::SystemTime::now();
    let mut time_constraints: Vec<TimeFilter> = Vec::new();
    if let Some(t) = matches.value_of("changed-within") {
        if let Some(f) = TimeFilter::after(&now, t) {
            time_constraints.push(f);
        } else {
            print_error_and_exit!("'{}' is not a valid date or duration. See 'fd --help'.", t);
        }
    }
    if let Some(t) = matches.value_of("changed-before") {
        if let Some(f) = TimeFilter::before(&now, t) {
            time_constraints.push(f);
        } else {
            print_error_and_exit!("'{}' is not a valid date or duration. See 'fd --help'.", t);
        }
    }

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
        one_file_system: matches.is_present("one-file-system"),
        null_separator: matches.is_present("null_separator"),
        prune: matches.is_present("prune"),
        max_depth: matches
            .value_of("depth")
            .or_else(|| matches.value_of("rg-depth"))
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
        file_types: matches.values_of("file-type").map(|values| {
            let mut file_types = FileTypes::default();
            for value in values {
                match value {
                    "f" | "file" => file_types.files = true,
                    "d" | "directory" => file_types.directories = true,
                    "l" | "symlink" => file_types.symlinks = true,
                    "x" | "executable" => {
                        file_types.executables_only = true;
                        file_types.files = true;
                    }
                    "e" | "empty" => {
                        file_types.empty_only = true;
                    }
                    _ => unreachable!(),
                }
            }

            // If only 'empty' was specified, search for both files and directories:
            if file_types.empty_only && !(file_types.files || file_types.directories) {
                file_types.files = true;
                file_types.directories = true;
            }

            file_types
        }),
        extensions: matches.values_of("extension").map(|exts| {
            let patterns = exts
                .map(|e| e.trim_start_matches('.'))
                .map(|e| format!(r".\.{}$", regex::escape(e)));
            match RegexSetBuilder::new(patterns)
                .case_insensitive(true)
                .build()
            {
                Ok(re) => re,
                Err(err) => {
                    print_error_and_exit!("{}", err.description());
                }
            }
        }),
        command: command.map(Arc::new),
        exclude_patterns: matches
            .values_of("exclude")
            .map(|v| v.map(|p| String::from("!") + p).collect())
            .unwrap_or_else(|| vec![]),
        ignore_files: matches
            .values_of("ignore-file")
            .map(|vs| vs.map(PathBuf::from).collect())
            .unwrap_or_else(|| vec![]),
        size_constraints: size_limits,
        time_constraints,
        show_filesystem_errors: matches.is_present("show-errors"),
        path_separator,
    };

    match RegexBuilder::new(&pattern_regex)
        .case_insensitive(!config.case_sensitive)
        .dot_matches_new_line(true)
        .build()
    {
        Ok(re) => {
            let exit_code = walk::scan(&dir_vec, Arc::new(re), Arc::new(config));
            process::exit(exit_code.into());
        }
        Err(err) => {
            print_error_and_exit!(
                "{}\nHint: You can use the '--fixed-strings' option to search for a \
                 literal string instead of a regular expression",
                err.description()
            );
        }
    }
}

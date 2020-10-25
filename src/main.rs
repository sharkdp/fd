mod app;
mod error;
mod exec;
mod exit_codes;
mod filesystem;
mod filetypes;
mod filter;
mod options;
mod output;
mod regex_helper;
mod walk;

use std::env;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::time;

use anyhow::{anyhow, Context, Result};
use atty::Stream;
use globset::GlobBuilder;
use lscolors::LsColors;
use regex::bytes::{RegexBuilder, RegexSetBuilder};

use crate::error::print_error;
use crate::exec::CommandTemplate;
use crate::exit_codes::ExitCode;
use crate::filetypes::FileTypes;
#[cfg(unix)]
use crate::filter::OwnerFilter;
use crate::filter::{SizeFilter, TimeFilter};
use crate::options::Options;
use crate::regex_helper::pattern_has_uppercase_char;

// We use jemalloc for performance reasons, see https://github.com/sharkdp/fd/pull/481
// FIXME: re-enable jemalloc on macOS, see comment in Cargo.toml file for more infos
#[cfg(all(
    not(windows),
    not(target_os = "android"),
    not(target_os = "macos"),
    not(target_env = "musl")
))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn run() -> Result<ExitCode> {
    let matches = app::build_app().get_matches_from(env::args_os());

    // Set the current working directory of the process
    if let Some(base_directory) = matches.value_of_os("base-directory") {
        let base_directory = Path::new(base_directory);
        if !filesystem::is_dir(base_directory) {
            return Err(anyhow!(
                "The '--base-directory' path '{}' is not a directory.",
                base_directory.to_string_lossy()
            ));
        }
        env::set_current_dir(base_directory).with_context(|| {
            format!(
                "Could not set '{}' as the current working directory",
                base_directory.to_string_lossy()
            )
        })?;
    }

    let current_directory = Path::new(".");
    if !filesystem::is_dir(current_directory) {
        return Err(anyhow!(
            "Could not retrieve current directory (has it been deleted?)."
        ));
    }

    // Get the search pattern
    let pattern = matches
        .value_of_os("pattern")
        .map(|p| {
            p.to_str()
                .ok_or_else(|| anyhow!("The search pattern includes invalid UTF-8 sequences."))
        })
        .transpose()?
        .unwrap_or("");

    // Get one or more root directories to search.
    let passed_arguments = matches
        .values_of_os("path")
        .or_else(|| matches.values_of_os("search-path"));

    let mut search_paths = if let Some(paths) = passed_arguments {
        let mut directories = vec![];
        for path in paths {
            let path_buffer = PathBuf::from(path);
            if filesystem::is_dir(&path_buffer) {
                directories.push(path_buffer);
            } else {
                print_error(format!(
                    "Search path '{}' is not a directory.",
                    path_buffer.to_string_lossy()
                ));
            }
        }

        directories
    } else {
        vec![current_directory.to_path_buf()]
    };

    // Check if we have no valid search paths.
    if search_paths.is_empty() {
        return Err(anyhow!("No valid search paths given."));
    }

    if matches.is_present("absolute-path") {
        search_paths = search_paths
            .iter()
            .map(|path_buffer| {
                path_buffer
                    .canonicalize()
                    .and_then(|pb| filesystem::absolute_path(pb.as_path()))
                    .unwrap()
            })
            .collect();
    }

    // Detect if the user accidentally supplied a path instead of a search pattern
    if !matches.is_present("full-path")
        && pattern.contains(std::path::MAIN_SEPARATOR)
        && filesystem::is_dir(Path::new(pattern))
    {
        return Err(anyhow!(
            "The search pattern '{pattern}' contains a path-separation character ('{sep}') \
             and will not lead to any search results.\n\n\
             If you want to search for all files inside the '{pattern}' directory, use a match-all pattern:\n\n  \
             fd . '{pattern}'\n\n\
             Instead, if you want your pattern to match the full file path, use:\n\n  \
             fd --full-path '{pattern}'",
            pattern = pattern,
            sep = std::path::MAIN_SEPARATOR,
        ));
    }

    let pattern_regex = if matches.is_present("glob") && !pattern.is_empty() {
        let glob = GlobBuilder::new(pattern).literal_separator(true).build()?;
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

    #[cfg(windows)]
    let ansi_colors_support =
        ansi_term::enable_ansi_support().is_ok() || std::env::var_os("TERM").is_some();

    #[cfg(not(windows))]
    let ansi_colors_support = true;

    let interactive_terminal = atty::is(Stream::Stdout);
    let colored_output = match matches.value_of("color") {
        Some("always") => true,
        Some("never") => false,
        _ => ansi_colors_support && env::var_os("NO_COLOR").is_none() && interactive_terminal,
    };

    let path_separator = matches.value_of("path-separator").map(|str| str.to_owned());

    let ls_colors = if colored_output {
        Some(LsColors::from_env().unwrap_or_default())
    } else {
        None
    };

    let command = if let Some(args) = matches.values_of("exec") {
        Some(CommandTemplate::new(args))
    } else if let Some(args) = matches.values_of("exec-batch") {
        Some(CommandTemplate::new_batch(args)?)
    } else if matches.is_present("list-details") {
        let color = matches.value_of("color").unwrap_or("auto");
        let color_arg = ["--color=", color].concat();

        #[allow(unused)]
        let gnu_ls = |command_name| {
            vec![
                command_name,
                "-l",               // long listing format
                "--human-readable", // human readable file sizes
                "--directory",      // list directories themselves, not their contents
                &color_arg,
            ]
        };

        let cmd: Vec<&str> = if cfg!(unix) {
            if !cfg!(any(
                target_os = "macos",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            )) {
                // Assume ls is GNU ls
                gnu_ls("ls")
            } else {
                // MacOS, DragonFlyBSD, FreeBSD
                use std::process::{Command, Stdio};

                // Use GNU ls, if available (support for --color=auto, better LS_COLORS support)
                let gnu_ls_exists = Command::new("gls")
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_ok();

                if gnu_ls_exists {
                    gnu_ls("gls")
                } else {
                    let mut cmd = vec![
                        "ls", // BSD version of ls
                        "-l", // long listing format
                        "-h", // '--human-readable' is not available, '-h' is
                        "-d", // '--directory' is not available, but '-d' is
                    ];

                    if !cfg!(any(target_os = "netbsd", target_os = "openbsd")) && colored_output {
                        // -G is not available in NetBSD's and OpenBSD's ls
                        cmd.push("-G");
                    }

                    cmd
                }
            }
        } else if cfg!(windows) {
            use std::process::{Command, Stdio};

            // Use GNU ls, if available
            let gnu_ls_exists = Command::new("ls")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok();

            if gnu_ls_exists {
                gnu_ls("ls")
            } else {
                return Err(anyhow!(
                    "'fd --list-details' is not supported on Windows unless GNU 'ls' is installed."
                ));
            }
        } else {
            return Err(anyhow!(
                "'fd --list-details' is not supported on this platform."
            ));
        };

        Some(CommandTemplate::new_batch(&cmd).unwrap())
    } else {
        None
    };

    let size_limits = if let Some(vs) = matches.values_of("size") {
        vs.map(|sf| {
            SizeFilter::from_string(sf)
                .ok_or_else(|| anyhow!("'{}' is not a valid size constraint. See 'fd --help'.", sf))
        })
        .collect::<Result<Vec<_>>>()?
    } else {
        vec![]
    };

    let now = time::SystemTime::now();
    let mut time_constraints: Vec<TimeFilter> = Vec::new();
    if let Some(t) = matches.value_of("changed-within") {
        if let Some(f) = TimeFilter::after(&now, t) {
            time_constraints.push(f);
        } else {
            return Err(anyhow!(
                "'{}' is not a valid date or duration. See 'fd --help'.",
                t
            ));
        }
    }
    if let Some(t) = matches.value_of("changed-before") {
        if let Some(f) = TimeFilter::before(&now, t) {
            time_constraints.push(f);
        } else {
            return Err(anyhow!(
                "'{}' is not a valid date or duration. See 'fd --help'.",
                t
            ));
        }
    }

    #[cfg(unix)]
    let owner_constraint = if let Some(s) = matches.value_of("owner") {
        OwnerFilter::from_string(s)?
    } else {
        None
    };

    let config = Options {
        case_sensitive,
        search_full_path: matches.is_present("full-path"),
        ignore_hidden: !(matches.is_present("hidden")
            || matches.occurrences_of("rg-alias-hidden-ignore") >= 2),
        read_fdignore: !(matches.is_present("no-ignore")
            || matches.is_present("rg-alias-hidden-ignore")),
        read_vcsignore: !(matches.is_present("no-ignore")
            || matches.is_present("rg-alias-hidden-ignore")
            || matches.is_present("no-ignore-vcs")),
        read_global_ignore: !(matches.is_present("no-ignore")
            || matches.is_present("rg-alias-hidden-ignore")
            || matches.is_present("no-global-ignore-file")),
        follow_links: matches.is_present("follow"),
        one_file_system: matches.is_present("one-file-system"),
        null_separator: matches.is_present("null_separator"),
        max_depth: matches
            .value_of("max-depth")
            .or_else(|| matches.value_of("rg-depth"))
            .or_else(|| matches.value_of("exact-depth"))
            .map(|n| usize::from_str_radix(n, 10))
            .transpose()
            .context("Failed to parse argument to --max-depth/--exact-depth")?,
        min_depth: matches
            .value_of("min-depth")
            .or_else(|| matches.value_of("exact-depth"))
            .map(|n| usize::from_str_radix(n, 10))
            .transpose()
            .context("Failed to parse argument to --min-depth/--exact-depth")?,
        prune: matches.is_present("prune"),
        threads: std::cmp::max(
            matches
                .value_of("threads")
                .map(|n| usize::from_str_radix(n, 10))
                .transpose()
                .context(format!("Failed to parse number of threads"))?
                .map(|n| {
                    if n > 0 {
                        Ok(n)
                    } else {
                        Err(anyhow!("Number of threads must be positive."))
                    }
                })
                .transpose()?
                .unwrap_or_else(num_cpus::get),
            1,
        ),
        max_buffer_time: matches
            .value_of("max-buffer-time")
            .map(|n| u64::from_str_radix(n, 10))
            .transpose()
            .context("Failed to parse max. buffer time argument")?
            .map(time::Duration::from_millis),
        ls_colors,
        interactive_terminal,
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
                    "e" | "empty" => file_types.empty_only = true,
                    "s" | "socket" => file_types.sockets = true,
                    "p" | "pipe" => file_types.pipes = true,
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
        extensions: matches
            .values_of("extension")
            .map(|exts| {
                let patterns = exts
                    .map(|e| e.trim_start_matches('.'))
                    .map(|e| format!(r".\.{}$", regex::escape(e)));
                RegexSetBuilder::new(patterns)
                    .case_insensitive(true)
                    .build()
            })
            .transpose()?,
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
        #[cfg(unix)]
        owner_constraint,
        show_filesystem_errors: matches.is_present("show-errors"),
        path_separator,
        max_results: matches
            .value_of("max-results")
            .map(|n| usize::from_str_radix(n, 10))
            .transpose()
            .context("Failed to parse --max-results argument")?
            .filter(|&n| n > 0)
            .or_else(|| {
                if matches.is_present("max-one-result") {
                    Some(1)
                } else {
                    None
                }
            }),
    };

    let re = RegexBuilder::new(&pattern_regex)
        .case_insensitive(!config.case_sensitive)
        .dot_matches_new_line(true)
        .build()
        .map_err(|e| {
            anyhow!(
                "{}\n\nNote: You can use the '--fixed-strings' option to search for a \
                 literal string instead of a regular expression. Alternatively, you can \
                 also use the '--glob' option to match on a glob pattern.",
                e.to_string()
            )
        })?;

    walk::scan(&search_paths, Arc::new(re), Arc::new(config))
}

fn main() {
    let result = run();
    match result {
        Ok(exit_code) => {
            process::exit(exit_code.into());
        }
        Err(err) => {
            eprintln!("[fd error]: {:#}", err);
            process::exit(ExitCode::GeneralError.into());
        }
    }
}

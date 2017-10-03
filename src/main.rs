#[macro_use]
extern crate clap;
extern crate ansi_term;
extern crate atty;
extern crate regex;
extern crate ignore;
extern crate num_cpus;

pub mod lscolors;
pub mod fshelper;

use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::io::Write;
use std::ops::Deref;
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf, Component};
use std::process;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;
use std::time;

use clap::{App, AppSettings, Arg};
use atty::Stream;
use regex::{Regex, RegexBuilder};
use ignore::WalkBuilder;

use lscolors::LsColors;

/// Defines how to display search result paths.
#[derive(PartialEq)]
enum PathDisplay {
    /// As an absolute path
    Absolute,

    /// As a relative path
    Relative
}

/// The type of file to search for.
#[derive(Copy, Clone)]
enum FileType {
    Any,
    RegularFile,
    Directory,
    SymLink
}

/// Configuration options for *fd*.
struct FdOptions {
    /// Determines whether the regex search is case-sensitive or case-insensitive.
    case_sensitive: bool,

    /// Whether to search within the full file path or just the base name (filename or directory
    /// name).
    search_full_path: bool,

    /// Whether to ignore hidden files and directories (or not).
    ignore_hidden: bool,

    /// Whether to respect VCS ignore files (`.gitignore`, `.ignore`, ..) or not.
    read_ignore: bool,

    /// Whether to follow symlinks or not.
    follow_links: bool,

    /// Whether elements of output should be separated by a null character
    null_separator: bool,

    /// The maximum search depth, or `None` if no maximum search depth should be set.
    ///
    /// A depth of `1` includes all files under the current directory, a depth of `2` also includes
    /// all files under subdirectories of the current directory, etc.
    max_depth: Option<usize>,

    /// The number of threads to use.
    threads: usize,

    /// Time to buffer results internally before streaming to the console. This is useful to
    /// provide a sorted output, in case the total execution time is shorter than
    /// `max_buffer_time`.
    max_buffer_time: Option<time::Duration>,

    /// Display results as relative or absolute path.
    path_display: PathDisplay,

    /// `None` if the output should not be colorized. Otherwise, a `LsColors` instance that defines
    /// how to style different filetypes.
    ls_colors: Option<LsColors>,

    /// The type of file to search for. All files other than the specified type will be ignored.
    file_type: FileType,

    /// The extension to search for. Only entries matching the extension will be included.
    ///
    /// The value (if present) will be a lowercase string without leading dots.
    extension: Option<String>,
}

/// The receiver thread can either be buffering results or directly streaming to the console.
enum ReceiverMode {
    /// Receiver is still buffering in order to sort the results, if the search finishes fast
    /// enough.
    Buffering,

    /// Receiver is directly printing results to the output.
    Streaming
}

/// Root directory
static ROOT_DIR : &'static str = "/";

/// Parent directory
static PARENT_DIR : &'static str = "..";

/// Print a search result to the console.
fn print_entry(base: &Path, entry: &PathBuf, config: &FdOptions) {
    let path_full = base.join(entry);

    let path_str = entry.to_string_lossy();

    #[cfg(target_family = "unix")]
    let is_executable = |p: Option<&std::fs::Metadata>| {
        p.map(|f| f.permissions().mode() & 0o111 != 0)
         .unwrap_or(false)
    };

    #[cfg(not(target_family = "unix"))]
    let is_executable = |_: Option<&std::fs::Metadata>| { false };

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    if let Some(ref ls_colors) = config.ls_colors {
        let default_style = ansi_term::Style::default();

        let mut component_path = base.to_path_buf();

        if config.path_display == PathDisplay::Absolute {
            print!("{}", ls_colors.directory.paint(ROOT_DIR));
        }

        // Traverse the path and colorize each component
        for component in entry.components() {
            let comp_str = match component {
                Component::Normal(p) => p.to_string_lossy(),
                Component::ParentDir => Cow::from(PARENT_DIR),
                _                    => error("Error: unexpected path component.")
            };

            component_path.push(Path::new(comp_str.deref()));

            let metadata = component_path.metadata().ok();
            let is_directory = metadata.as_ref().map(|md| md.is_dir()).unwrap_or(false);

            let style =
                if component_path.symlink_metadata()
                                 .map(|md| md.file_type().is_symlink())
                                 .unwrap_or(false) {
                    &ls_colors.symlink
                } else if is_directory {
                    &ls_colors.directory
                } else if is_executable(metadata.as_ref()) {
                    &ls_colors.executable
                } else {
                    // Look up file name
                    let o_style =
                        component_path.file_name()
                                      .and_then(|n| n.to_str())
                                      .and_then(|n| ls_colors.filenames.get(n));

                    match o_style {
                        Some(s) => s,
                        None =>
                            // Look up file extension
                            component_path.extension()
                                          .and_then(|e| e.to_str())
                                          .and_then(|e| ls_colors.extensions.get(e))
                                          .unwrap_or(&default_style)
                    }
                };

            write!(handle, "{}", style.paint(comp_str)).ok();

            if is_directory && component_path != path_full {
                let sep = std::path::MAIN_SEPARATOR.to_string();
                write!(handle, "{}", style.paint(sep)).ok();
            }
        }

        let r = if config.null_separator {
          write!(handle, "\0")
        } else {
          writeln!(handle, "")
        };
        if r.is_err() {
            // Probably a broken pipe. Exit gracefully.
            process::exit(0);
        }
    } else {
        // Uncolorized output

        let prefix = if config.path_display == PathDisplay::Absolute { ROOT_DIR } else { "" };
        let separator = if config.null_separator { "\0" } else { "\n" };

        let r = write!(&mut std::io::stdout(), "{}{}{}", prefix, path_str, separator);

        if r.is_err() {
            // Probably a broken pipe. Exit gracefully.
            process::exit(0);
        }
    }
}

/// Recursively scan the given search path and search for files / pathnames matching the pattern.
fn scan(root: &Path, pattern: Arc<Regex>, base: &Path, config: Arc<FdOptions>) {
    let (tx, rx) = channel();

    let walker = WalkBuilder::new(root)
                     .hidden(config.ignore_hidden)
                     .ignore(config.read_ignore)
                     .git_ignore(config.read_ignore)
                     .parents(config.read_ignore)
                     .git_global(config.read_ignore)
                     .git_exclude(config.read_ignore)
                     .follow_links(config.follow_links)
                     .max_depth(config.max_depth)
                     .threads(config.threads)
                     .build_parallel();

    // Spawn the thread that receives all results through the channel.
    let rx_config = Arc::clone(&config);
    let rx_base = base.to_owned();
    let receiver_thread = thread::spawn(move || {
        let start = time::Instant::now();

        let mut buffer = vec!();

        // Start in buffering mode
        let mut mode = ReceiverMode::Buffering;

        // Maximum time to wait before we start streaming to the console.
        let max_buffer_time = rx_config.max_buffer_time
                                       .unwrap_or_else(|| time::Duration::from_millis(100));

        for value in rx {
            match mode {
                ReceiverMode::Buffering => {
                    buffer.push(value);

                    // Have we reached the maximum time?
                    if time::Instant::now() - start > max_buffer_time {
                        // Flush the buffer
                        for v in &buffer {
                            print_entry(&rx_base, v, &rx_config);
                        }
                        buffer.clear();

                        // Start streaming
                        mode = ReceiverMode::Streaming;
                    }
                },
                ReceiverMode::Streaming => {
                    print_entry(&rx_base, &value, &rx_config);
                }
            }
        }

        // If we have finished fast enough (faster than max_buffer_time), we haven't streamed
        // anything to the console, yet. In this case, sort the results and print them:
        if !buffer.is_empty() {
            buffer.sort();
            for value in buffer {
                print_entry(&rx_base, &value, &rx_config);
            }
        }
    });

    // Spawn the sender threads.
    walker.run(|| {
        let base = base.to_owned();
        let config = Arc::clone(&config);
        let pattern = Arc::clone(&pattern);
        let tx_thread = tx.clone();

        Box::new(move |entry_o| {
            let entry = match entry_o {
                Ok(e) => e,
                Err(_) => return ignore::WalkState::Continue
            };

            // Filter out unwanted file types.
            match config.file_type {
                FileType::Any => (),
                FileType::RegularFile => if entry.file_type().map_or(false, |ft| !ft.is_file()) {
                    return ignore::WalkState::Continue;
                },
                FileType::Directory => if entry.file_type().map_or(false, |ft| !ft.is_dir()) {
                    return ignore::WalkState::Continue;
                },
                FileType::SymLink => if entry.file_type().map_or(false, |ft| !ft.is_symlink()) {
                    return ignore::WalkState::Continue;
                },
            }

            // Filter out unwanted extensions.
            if let Some(ref filter_ext) = config.extension {
                let entry_ext = entry.path().extension().map(|e| e.to_string_lossy().to_lowercase());
                if entry_ext.map_or(true, |ext| ext != *filter_ext) {
                    return ignore::WalkState::Continue;
                }
            }

            let path_rel_buf = match fshelper::path_relative_from(entry.path(), &*base) {
                Some(p) => p,
                None => error("Error: could not get relative path for directory entry.")
            };
            let path_rel = path_rel_buf.as_path();

            let search_str_o =
                if config.search_full_path {
                    Some(path_rel.to_string_lossy())
                } else {
                    path_rel.file_name()
                            .map(|f| f.to_string_lossy())
                };

            if let Some(search_str) = search_str_o {
                // TODO: take care of the unwrap call
                pattern.find(&*search_str)
                       .map(|_| tx_thread.send(path_rel_buf.to_owned()).unwrap());
            }

            ignore::WalkState::Continue
        })
    });

    // Drop the initial sender. If we don't do this, the receiver will block even
    // if all threads have finished, since there is still one sender around.
    drop(tx);

    // Wait for the receiver thread to print out all results.
    receiver_thread.join().unwrap();
}

/// Print error message to stderr and exit with status `1`.
fn error(message: &str) -> ! {
    writeln!(&mut std::io::stderr(), "{}", message)
        .expect("Failed writing to stderr");
    process::exit(1);
}

fn main() {
    let matches =
        App::new("fd")
            .version(crate_version!())
            .usage("fd [FLAGS/OPTIONS] [<pattern>] [<path>]")
            .setting(AppSettings::ColoredHelp)
            .setting(AppSettings::DeriveDisplayOrder)
            .arg(Arg::with_name("hidden")
                        .long("hidden")
                        .short("H")
                        .help("Search hidden files and directories"))
            .arg(Arg::with_name("no-ignore")
                        .long("no-ignore")
                        .short("I")
                        .help("Do not respect .(git)ignore files"))
            .arg(Arg::with_name("case-sensitive")
                        .long("case-sensitive")
                        .short("s")
                        .help("Case-sensitive search (default: smart case)"))
            .arg(Arg::with_name("absolute-path")
                        .long("absolute-path")
                        .short("a")
                        .help("Show absolute instead of relative paths"))
            .arg(Arg::with_name("follow")
                        .long("follow")
                        .short("L")
                        .alias("dereference")
                        .help("Follow symbolic links"))
            .arg(Arg::with_name("full-path")
                        .long("full-path")
                        .short("p")
                        .help("Search full path (default: file-/dirname only)"))
            .arg(Arg::with_name("null_separator")
                        .long("print0")
                        .short("0")
                        .help("Separate results by the null character"))
            .arg(Arg::with_name("depth")
                        .long("max-depth")
                        .short("d")
                        .takes_value(true)
                        .help("Set maximum search depth (default: none)"))
            .arg(Arg::with_name("file-type")
                        .long("type")
                        .short("t")
                        .takes_value(true)
                        .possible_values(&["f", "file", "d", "directory", "s", "symlink"])
                        .hide_possible_values(true)
                        .help("Filter by type: f(ile), d(irectory), s(ymlink)"))
            .arg(Arg::with_name("extension")
                        .long("extension")
                        .short("e")
                        .takes_value(true)
                        .value_name("ext")
                        .help("Filter by file extension"))
            .arg(Arg::with_name("color")
                        .long("color")
                        .short("c")
                        .takes_value(true)
                        .possible_values(&["never", "auto", "always"])
                        .hide_possible_values(true)
                        .help("When to use color in the output:\n\
                               never, auto, always (default: auto)"))
            .arg(Arg::with_name("threads")
                        .long("threads")
                        .short("j")
                        .takes_value(true)
                        .help("Set number of threads to use for searching\n\
                               (default: number of available CPU cores)"))
            .arg(Arg::with_name("max-buffer-time")
                        .long("max-buffer-time")
                        .takes_value(true)
                        .hidden(true)
                        .help("the time (in ms) to buffer, before streaming to the console"))
            .arg(Arg::with_name("pattern")
                        .help("the search pattern, a regular expression (optional)"))
            .arg(Arg::with_name("path")
                        .help("the root directory for the filesystem search (optional)"))
            .get_matches();

    // Get the search pattern
    let empty_pattern = String::new();
    let pattern = matches.value_of("pattern").unwrap_or(&empty_pattern);

    // Get the current working directory
    let current_dir_buf = match env::current_dir() {
        Ok(cd) => cd,
        Err(_) => error("Error: could not get current directory.")
    };
    let current_dir = current_dir_buf.as_path();

    // Get the root directory for the search
    let mut root_dir_is_absolute = false;
    let root_dir_buf = if let Some(rd) = matches.value_of("path") {
        let path = Path::new(rd);

        root_dir_is_absolute = path.is_absolute();

        path.canonicalize().unwrap_or_else(
            |_| error(&format!("Error: could not find directory '{}'.", rd))
        )
    } else {
        current_dir_buf.clone()
    };

    if !root_dir_buf.is_dir() {
        error(&format!("Error: '{}' is not a directory.", root_dir_buf.to_string_lossy()));
    }

    let root_dir = root_dir_buf.as_path();

    // The search will be case-sensitive if the command line flag is set or
    // if the pattern has an uppercase character (smart case).
    let case_sensitive = matches.is_present("case-sensitive") ||
                         pattern.chars().any(char::is_uppercase);

    let colored_output = match matches.value_of("color") {
        Some("always") => true,
        Some("never") => false,
        _ => atty::is(Stream::Stdout)
    };

    let ls_colors =
        if colored_output {
            Some(
                env::var("LS_COLORS")
                    .ok()
                    .map(|val| LsColors::from_string(&val))
                    .unwrap_or_default()
            )
        } else {
            None
        };

    let config = FdOptions {
        case_sensitive:    case_sensitive,
        search_full_path:  matches.is_present("full-path"),
        ignore_hidden:     !matches.is_present("hidden"),
        read_ignore:       !matches.is_present("no-ignore"),
        follow_links:      matches.is_present("follow"),
        null_separator:    matches.is_present("null_separator"),
        max_depth:         matches.value_of("depth")
                                   .and_then(|n| usize::from_str_radix(n, 10).ok()),
        threads:           std::cmp::max(
                             matches.value_of("threads")
                                    .and_then(|n| usize::from_str_radix(n, 10).ok())
                                    .unwrap_or_else(num_cpus::get),
                             1
                           ),
        max_buffer_time:   matches.value_of("max-buffer-time")
                                  .and_then(|n| u64::from_str_radix(n, 10).ok())
                                  .map(time::Duration::from_millis),
        path_display:      if matches.is_present("absolute-path") || root_dir_is_absolute {
                               PathDisplay::Absolute
                           } else {
                               PathDisplay::Relative
                           },
        ls_colors:         ls_colors,
        file_type:         match matches.value_of("file-type") {
                               Some("f") | Some("file") => FileType::RegularFile,
                               Some("d") | Some("directory") => FileType::Directory,
                               Some("s") | Some("symlink") => FileType::SymLink,
                               _  => FileType::Any,
                           },
        extension:         matches.value_of("extension")
                                  .map(|e| e.trim_left_matches('.').to_lowercase()),
    };

    let root = Path::new(ROOT_DIR);
    let base = match config.path_display {
        PathDisplay::Relative => current_dir,
        PathDisplay::Absolute => root
    };

    match RegexBuilder::new(pattern)
              .case_insensitive(!config.case_sensitive)
              .build() {
        Ok(re)   => scan(root_dir, Arc::new(re), base, Arc::new(config)),
        Err(err) => error(err.description())
    }
}

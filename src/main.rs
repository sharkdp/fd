extern crate ansi_term;
extern crate getopts;
extern crate isatty;
extern crate regex;
extern crate ignore;

pub mod lscolors;
pub mod fshelper;

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, Component};
use std::process;

use getopts::Options;
use isatty::stdout_isatty;
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

    /// The maximum search depth, or `None` if no maximum search depth should be set.
    ///
    /// A depth of `1` includes all files under the current directory, a depth of `2` also includes
    /// all files under subdirectories of the current directory, etc.
    max_depth: Option<usize>,

    /// Display results as relative or absolute path.
    path_display: PathDisplay,

    /// `None` if the output should not be colorized. Otherwise, a `LsColors` instance that defines
    /// how to style different filetypes.
    ls_colors: Option<LsColors>
}

/// Root directory
static ROOT_DIR : &'static str = "/";

/// Print a search result to the console.
fn print_entry(base: &Path, entry: &Path, config: &FdOptions) {
    let path_full = base.join(entry);

    let path_str = match entry.to_str() {
        Some(p) => p,
        None    => return
    };

    #[cfg(target_os = "unix")]
    let is_executable = |p: &std::path::PathBuf| {
        p.metadata()
         .ok()
         .map(|f| f.permissions().mode() & 0o111 != 0)
         .unwrap_or(false)
    };

    #[cfg(not(target_os = "unix"))]
    let is_executable =  |p: &std::path::PathBuf| {false};

    if let Some(ref ls_colors) = config.ls_colors {
        let mut component_path = base.to_path_buf();

        if config.path_display == PathDisplay::Absolute {
            print!("{}", ls_colors.directory.paint(ROOT_DIR));
        }

        // Traverse the path and colorize each component
        for component in entry.components() {
            let comp_str = match component {
                Component::Normal(p) => p.to_str().unwrap(),
                Component::ParentDir => "..",
                _                    => error("Unexpected path component")
            };

            component_path.push(Path::new(comp_str));

            let style =
                if component_path.symlink_metadata()
                                 .map(|md| md.file_type().is_symlink())
                                 .unwrap_or(false) {
                    ls_colors.symlink
                } else if component_path.is_dir() {
                    ls_colors.directory
                } else if is_executable(&component_path) {
                    ls_colors.executable
                } else {
                    // Look up file name
                    let o_style =
                        component_path.file_name()
                                      .and_then(|n| n.to_str())
                                      .and_then(|n| ls_colors.filenames.get(n));

                    match o_style {
                        Some(s) => *s,
                        None =>
                            // Look up file extension
                            component_path.extension()
                                          .and_then(|e| e.to_str())
                                          .and_then(|e| ls_colors.extensions.get(e))
                                          .cloned()
                                          .unwrap_or_default()
                    }
                };

            print!("{}", style.paint(comp_str));

            if component_path.is_dir() && component_path != path_full {
                let sep = std::path::MAIN_SEPARATOR.to_string();
                print!("{}", style.paint(sep));
            }
        }
        println!();
    } else {
        // Uncolorized output:

        if config.path_display == PathDisplay::Absolute {
            print!("{}", ROOT_DIR);
        }
        println!("{}", path_str);
    }
}

/// Recursively scan the given search path and search for files / pathnames matching the pattern.
fn scan(root: &Path, pattern: &Regex, base: &Path, config: &FdOptions) {
    let walker = WalkBuilder::new(root)
                     .hidden(config.ignore_hidden)
                     .ignore(config.read_ignore)
                     .git_ignore(config.read_ignore)
                     .parents(config.read_ignore)
                     .git_global(config.read_ignore)
                     .git_exclude(config.read_ignore)
                     .follow_links(config.follow_links)
                     .max_depth(config.max_depth)
                     .build()
                     .into_iter()
                     .filter_map(|e| e.ok())
                     .filter(|e| e.path() != root);

    for entry in walker {
        let path_rel_buf = match fshelper::path_relative_from(entry.path(), base) {
            Some(p) => p,
            None => error("Could not get relative path for directory entry.")
        };
        let path_rel = path_rel_buf.as_path();

        let search_str =
            if config.search_full_path {
                path_rel.to_str()
            } else {
                path_rel.file_name()
                        .and_then(OsStr::to_str)
            };

        search_str.and_then(|s| pattern.find(s))
                  .map(|_| print_entry(base, path_rel, config));
    }
}

/// Print error message to stderr and exit with status `1`.
fn error(message: &str) -> ! {
    writeln!(&mut std::io::stderr(), "{}", message)
        .expect("Failed writing to stderr");
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optflag("h", "help",
                      "print this help message");
    opts.optflag("s", "sensitive",
                      "case-sensitive search (default: smart case)");
    opts.optflag("p", "full-path",
                      "search full path (default: file-/dirname only)");
    opts.optflag("H", "hidden",
                      "search hidden files/directories");
    opts.optflag("I", "no-ignore",
                      "do not respect .(git)ignore files");
    opts.optflag("f", "follow",
                      "follow symlinks");
    opts.optflag("n", "no-color",
                      "do not colorize output");
    opts.optopt("d", "max-depth",
                     "maximum search depth (default: none)", "D");
    opts.optflag("a", "absolute",
                      "show absolute instead of relative paths");

    let matches = match opts.parse(&args[1..]) {
        Ok(m)  => m,
        Err(e) => error(e.description())
    };

    if matches.opt_present("h") {
        let brief = "Usage: fd [options] [PATTERN] [PATH]";
        print!("{}", opts.usage(brief));
        process::exit(1);
    }

    // Get the search pattern
    let empty_pattern = String::new();
    let pattern = matches.free.get(0).unwrap_or(&empty_pattern);

    // Get the current working directory
    let current_dir_buf = match env::current_dir() {
        Ok(cd) => cd,
        Err(_) => error("Could not get current directory!")
    };
    let current_dir = current_dir_buf.as_path();

    // Get the root directory for the search
    let root_dir_buf = matches.free.get(1)
                                   .and_then(|r| fs::canonicalize(r).ok())
                                   .unwrap_or_else(|| current_dir_buf.clone());
    let root_dir = root_dir_buf.as_path();

    // The search will be case-sensitive if the command line flag is set or
    // if the pattern has an uppercase character (smart case).
    let case_sensitive = matches.opt_present("sensitive") ||
                         pattern.chars().any(char::is_uppercase);

    let colored_output = !matches.opt_present("no-color") &&
                         stdout_isatty();

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
        search_full_path:  matches.opt_present("full-path"),
        ignore_hidden:     !matches.opt_present("hidden"),
        read_ignore:       !matches.opt_present("no-ignore"),
        follow_links:      matches.opt_present("follow"),
        max_depth:         matches.opt_str("max-depth")
                                   .and_then(|ds| usize::from_str_radix(&ds, 10).ok()),
        path_display:      if matches.opt_present("absolute") {
                               PathDisplay::Absolute
                           } else {
                               PathDisplay::Relative
                           },
        ls_colors:         ls_colors
    };

    let root = Path::new(ROOT_DIR);
    let base = match config.path_display {
        PathDisplay::Relative => current_dir,
        PathDisplay::Absolute => root
    };

    match RegexBuilder::new(pattern)
              .case_insensitive(!config.case_sensitive)
              .build() {
        Ok(re)   => scan(root_dir, &re, base, &config),
        Err(err) => error(err.description())
    }
}

extern crate ansi_term;
extern crate getopts;
extern crate isatty;
extern crate regex;
extern crate ignore;
extern crate fd;

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, Component};
use std::process;

use getopts::Options;
use isatty::stdout_isatty;
use regex::{Regex, RegexBuilder};
use ignore::WalkBuilder;

use fd::lscolors::LsColors;

/// Configuration options for *fd*.
struct FdOptions {
    case_sensitive: bool,
    search_full_path: bool,
    ignore_hidden: bool,
    read_ignore: bool,
    follow_links: bool,
    max_depth: Option<usize>,
    colored: bool,
    ls_colors: LsColors
}

/// Print a search result to the console.
fn print_entry(path_root: &Path, path_entry: &Path, config: &FdOptions) {
    let path_full = path_root.join(path_entry);

    let path_str = match path_entry.to_str() {
        Some(p) => p,
        None    => return
    };

    if !config.colored {
        println!("{}", path_str);
    } else {
        let mut component_path = path_root.to_path_buf();

        // Traverse the path and colorize each component
        for component in path_entry.components() {
            let comp_str = match component {
                Component::Normal(p) => p,
                _                    => error("Unexpected path component")
            };

            component_path.push(Path::new(comp_str));

            let style =
                if component_path.symlink_metadata()
                                 .map(|md| md.file_type().is_symlink())
                                 .unwrap_or(false) {
                    config.ls_colors.symlink
                } else if component_path.is_dir() {
                    config.ls_colors.directory
                } else {
                    // Look up file name
                    let o_style =
                        component_path.file_name()
                                      .and_then(|n| n.to_str())
                                      .and_then(|n| config.ls_colors.filenames.get(n));

                    match o_style {
                        Some(s) => *s,
                        None =>
                            // Look up file extension
                            component_path.extension()
                                          .and_then(|e| e.to_str())
                                          .and_then(|e| config.ls_colors.extensions.get(e))
                                          .cloned()
                                          .unwrap_or_default()
                    }
                };

            print!("{}", style.paint(comp_str.to_str().unwrap()));

            if component_path.is_dir() && component_path != path_full {
                let sep = std::path::MAIN_SEPARATOR.to_string();
                print!("{}", style.paint(sep));
            }
        }
        println!();
    }
}

/// Recursively scan the given root path and search for files / pathnames
/// matching the pattern.
fn scan(root: &Path, pattern: &Regex, config: &FdOptions) {
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
        let path_rel = match entry.path().strip_prefix(root) {
            Ok(p) => p,
            Err(_) => continue
        };

        let search_str =
            if config.search_full_path {
                path_rel.to_str()
            } else {
                path_rel.file_name()
                        .and_then(OsStr::to_str)
            };

        search_str.and_then(|s| pattern.find(s))
                  .map(|_| print_entry(root, path_rel, config));
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
    opts.optopt( "d", "max-depth",
                      "maximum search depth (default: none)", "D");

    let matches = match opts.parse(&args[1..]) {
        Ok(m)  => m,
        Err(e) => error(e.description())
    };

    if matches.opt_present("h") {
        let brief = "Usage: fd [options] [PATTERN]";
        print!("{}", opts.usage(brief));
        process::exit(1);
    }

    let empty = String::new();
    let pattern = matches.free.get(0).unwrap_or(&empty);

    let current_dir_buf = match env::current_dir() {
        Ok(cd) => cd,
        Err(_) => error("Could not get current directory!")
    };
    let current_dir = current_dir_buf.as_path();

    let colored_output = !matches.opt_present("no-color") &&
                         stdout_isatty();

    let ls_colors =
        env::var("LS_COLORS")
            .ok()
            .map(|val| LsColors::from_string(&val))
            .unwrap_or_default();

    let config = FdOptions {
        // The search will be case-sensitive if the command line flag is set or
        // if the pattern has an uppercase character (smart case).
        case_sensitive:    matches.opt_present("sensitive") ||
                           pattern.chars().any(char::is_uppercase),
        search_full_path:  matches.opt_present("full-path"),
        ignore_hidden:     !matches.opt_present("hidden"),
        read_ignore:       !matches.opt_present("no-ignore"),
        follow_links:      matches.opt_present("follow"),
        max_depth:
            matches.opt_str("max-depth")
                   .and_then(|ds| usize::from_str_radix(&ds, 10).ok()),
        colored:           colored_output,
        ls_colors:         ls_colors
    };

    match RegexBuilder::new(pattern)
              .case_insensitive(!config.case_sensitive)
              .build() {
        Ok(re)   => scan(current_dir, &re, &config),
        Err(err) => error(err.description())
    }
}

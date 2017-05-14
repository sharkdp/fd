extern crate ansi_term;
extern crate getopts;
extern crate isatty;
extern crate regex;
extern crate walkdir;

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;
use std::process;

use ansi_term::Colour;
use getopts::Options;
use isatty::stdout_isatty;
use regex::{Regex, RegexBuilder};
use walkdir::{WalkDir, DirEntry, WalkDirIterator};

struct FdOptions {
    case_sensitive: bool,
    search_full_path: bool,
    search_hidden: bool,
    follow_links: bool,
    colored: bool,
    max_depth: usize
}

const MAX_DEPTH_DEFAULT : usize = 25;

/// Print a search result to the console.
fn print_entry(entry: &DirEntry, path_rel: &Path, config: &FdOptions) {
    let path_str = match path_rel.to_str() {
        Some(p) => p,
        None    => return
    };

    if config.colored {
        let style = match entry {
            e if e.path_is_symbolic_link() => Colour::Purple,
            e if e.path().is_dir()         => Colour::Cyan,
            _                              => Colour::White
        };
        println!("{}", style.paint(path_str));
    } else {
        println!("{}", path_str);
    }
}

/// Check if filename of entry starts with a dot.
fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}

/// Recursively scan the given root path and search for files / pathnames
/// matching the pattern.
fn scan(root: &Path, pattern: &Regex, config: &FdOptions) {
    let walker = WalkDir::new(root)
                     .follow_links(config.follow_links)
                     .max_depth(config.max_depth)
                     .into_iter()
                     .filter_entry(|e| config.search_hidden || !is_hidden(e))
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
                if !path_rel.is_file() { continue }

                path_rel.file_name()
                        .and_then(OsStr::to_str)
            };

        search_str.and_then(|s| pattern.find(s))
                  .map(|_| print_entry(&entry, path_rel, &config));
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
    opts.optflag("h", "help", "print this help message");
    opts.optflag("s", "sensitive",
                      "case-sensitive search (default: smart case)");
    opts.optflag("f", "filename",
                      "search filenames only (default: full path)");
    opts.optflag("H", "hidden",
                      "search hidden files/directories (default: off)");
    opts.optflag("F", "follow", "follow symlinks (default: off)");
    opts.optflag("n", "no-color", "do not colorize output (default: on)");
    opts.optopt("d", "max-depth",
                     "maximum search depth (default: 25)", "D");

    let matches = match opts.parse(&args[1..]) {
        Ok(m)  => m,
        Err(e) => error(e.description())
    };

    if matches.opt_present("h") {
        let brief = "Usage: fd [options] [PATTERN]";
        print!("{}", opts.usage(&brief));
        process::exit(1);
    }

    let empty = String::new();
    let pattern = matches.free.get(0).unwrap_or(&empty);

    let current_dir_buf = match env::current_dir() {
        Ok(cd) => cd,
        Err(_) => error("Could not get current directory!")
    };
    let current_dir = current_dir_buf.as_path();


    let config = FdOptions {
        // The search will be case-sensitive if the command line flag is set or
        // if the pattern has an uppercase character (smart case).
        case_sensitive:    matches.opt_present("sensitive") ||
                           pattern.chars().any(char::is_uppercase),
        search_full_path: !matches.opt_present("filename"),
        search_hidden:     matches.opt_present("hidden"),
        colored:          !matches.opt_present("no-color") &&
                           stdout_isatty(),
        follow_links:      matches.opt_present("follow"),
        max_depth:
            matches.opt_str("max-depth")
                   .and_then(|ds| usize::from_str_radix(&ds, 10).ok())
                   .unwrap_or(MAX_DEPTH_DEFAULT)
    };

    match RegexBuilder::new(pattern)
              .case_insensitive(!config.case_sensitive)
              .build() {
        Ok(re)   => scan(&current_dir, &re, &config),
        Err(err) => error(err.description())
    }
}

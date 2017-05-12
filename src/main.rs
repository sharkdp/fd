extern crate walkdir;
extern crate regex;
extern crate getopts;
extern crate ansi_term;

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;
use std::process;

use walkdir::{WalkDir, DirEntry, WalkDirIterator};
use regex::{Regex, RegexBuilder};
use getopts::Options;
use ansi_term::Colour;

struct FdOptions {
    case_sensitive: bool,
    search_full_path: bool,
    follow_links: bool,
    colored: bool
}

/// Print a search result to the console.
fn print_entry(entry: &DirEntry, path_str: &str, config: &FdOptions) {
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
                     .into_iter()
                     .filter_entry(|e| !is_hidden(e))
                     .filter_map(|e| e.ok())
                     .filter(|e| e.path() != root);

    for entry in walker {
        if let Ok(path_relative) = entry.path().strip_prefix(root) {
            if let Some(path_str) = path_relative.to_str() {
                let haystack = if config.search_full_path {
                                   path_str
                               } else {
                                   path_relative.file_name()
                                                .and_then(OsStr::to_str)
                                                .unwrap()
                               };
                pattern.find(haystack)
                       .map(|_| print_entry(&entry, path_str, &config));
            }
        }
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
    opts.optflag("s", "sensitive", "case-sensitive search (default: smart case)");
    opts.optflag("f", "full-path", "search full path (default: only filename)");
    opts.optflag("F", "follow", "follow symlinks (default: off)");
    opts.optflag("n", "no-color", "do not colorize output");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => error(e.description())
    };

    if matches.opt_present("h") {
        let brief = "Usage: fd [PATTERN]";
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
        // The search will be case-sensitive if the command line flag is set or if
        // the pattern has an uppercase character (smart case).
        case_sensitive:    matches.opt_present("s") ||
                           pattern.chars().any(char::is_uppercase),
        search_full_path:  matches.opt_present("f"),
        colored:          !matches.opt_present("n"),
        follow_links:      matches.opt_present("F")
    };

    match RegexBuilder::new(pattern)
              .case_insensitive(!config.case_sensitive)
              .build() {
        Ok(re) => scan(&current_dir, &re, &config),
        Err(err) => error(err.description())
    }
}

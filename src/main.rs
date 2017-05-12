extern crate walkdir;
extern crate regex;
extern crate getopts;
extern crate ansi_term;

use std::env;
use std::path::Path;
use std::io::Write;
use std::process;
use std::error::Error;

use walkdir::{WalkDir, DirEntry, WalkDirIterator};
use regex::{Regex, RegexBuilder};
use getopts::Options;
use ansi_term::Colour;

/// Print a search result to the console.
fn print_entry(entry: &DirEntry, path_str: &str) {
    let style = match entry {
        e if e.path_is_symbolic_link() => Colour::Purple,
        e if e.path().is_dir()         => Colour::Cyan,
        _                              => Colour::White
    };
    println!("{}", style.paint(path_str));
}

/// Check if filename of entry starts with a dot.
fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}

/// Recursively scan the given root path and search for pathnames matching the
/// pattern.
fn scan(root: &Path, pattern: &Regex) {
    let walker = WalkDir::new(root).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e))
                       .filter_map(|e| e.ok()) {
        if entry.path() == root {
            continue;
        }

        let path_relative =
            match entry.path().strip_prefix(root) {
                Ok(r) => r,
                Err(_) => continue
            };

        let path_str = match path_relative.to_str() {
            Some(p) => p,
            None => continue
        };

        pattern.find(path_str)
               .map(|_| print_entry(&entry, path_str));
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
    opts.optflag("s", "sensitive", "case-sensitive search");
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

    // The search will be case-sensitive if the command line flag is set or if
    // the pattern has an uppercase character (smart case).
    let case_sensitive =
        matches.opt_present("s") ||
        pattern.chars().any(char::is_uppercase);

    match RegexBuilder::new(pattern)
              .case_insensitive(!case_sensitive)
              .build() {
        Ok(re) => scan(current_dir, &re),
        Err(err) => error(err.description())
    }
}

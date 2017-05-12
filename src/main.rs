extern crate walkdir;
extern crate regex;
extern crate getopts;

use std::env;
use std::path::Path;
use std::io::Write;
use std::process;
use std::error::Error;

use walkdir::{WalkDir, DirEntry, WalkDirIterator};
use regex::{Regex, RegexBuilder};
use getopts::Options;

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

        let path_str_r = entry
                            .path()
                            .strip_prefix(root) // create relative path
                            .ok()
                            .and_then(Path::to_str);

        let path_str = match path_str_r {
            Some(p) => p,
            None => continue
        };

        pattern.find(path_str)
               .map(|_| println!("{}", path_str));
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

    let case_insensitive = !matches.opt_present("s");

    let empty = String::new();
    let pattern = matches.free.get(0).unwrap_or(&empty);

    let current_dir_buf = match env::current_dir() {
        Ok(cd) => cd,
        Err(_) => error("Could not get current directory!")
    };
    let current_dir = current_dir_buf.as_path();

    match RegexBuilder::new(pattern)
              .case_insensitive(case_insensitive)
              .build() {
        Ok(re) => scan(current_dir, &re),
        Err(err) => error(err.description())
    }
}

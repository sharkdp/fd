extern crate walkdir;
extern crate regex;
extern crate getopts;

use std::env;
use std::path::Path;
use std::io::Write;
use std::process;
use std::error::Error;

use walkdir::{WalkDir, DirEntry, WalkDirIterator};
use regex::Regex;
use getopts::Options;

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}

fn scan(root: &Path, pattern: &Regex) {
    let walker = WalkDir::new(root).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e))
                       .filter_map(|e| e.ok()) {
        if entry.path() == root {
            continue;
        }

        let path_relative = entry.path().strip_prefix(root).unwrap();
        let path_str = match path_relative.to_str() {
            Some(s) => s,
            None => continue
        };
        match pattern.find(path_str) {
            Some(_) =>
                println!("{}", path_str),
            None =>
                continue
        }
    }
}

fn error<T: Error>(err: &T) -> ! {
    writeln!(&mut std::io::stderr(), "{}", err.description())
        .expect("Failed writing to stderr");
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help message");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { error(&f) }
    };

    if matches.opt_present("h") {
        let brief = "Usage: fd [PATTERN]";
        print!("{}", opts.usage(&brief));
        process::exit(1);
    }

    let empty = String::new();
    let pattern = matches.free.get(0).unwrap_or(&empty);

    let current_dir_buf = env::current_dir()
            .expect("Could not get current directory!");
    let current_dir = current_dir_buf.as_path();

    match Regex::new(pattern) {
        Ok(re) =>
            scan(current_dir, &re),
        Err(err) => error(&err)
    }
}

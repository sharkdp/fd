extern crate ansi_term;
extern crate getopts;
extern crate isatty;
extern crate regex;
extern crate walkdir;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Write, BufReader,BufRead};
use std::path::{Path, Component};
use std::process;

use ansi_term::Colour;
use getopts::Options;
use isatty::stdout_isatty;
use regex::{Regex, RegexBuilder};
use walkdir::{WalkDir, DirEntry, WalkDirIterator};

/// Maps file extensions to ANSI colors / styles.
type ExtensionStyles = HashMap<String, ansi_term::Style>;

/// Configuration options for *fd*.
struct FdOptions {
    case_sensitive: bool,
    search_full_path: bool,
    search_hidden: bool,
    follow_links: bool,
    colored: bool,
    max_depth: usize,
    extension_styles: Option<ExtensionStyles>
}

/// The default maximum recursion depth.
const MAX_DEPTH_DEFAULT : usize = 25;

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
                    Colour::Cyan.normal()
                } else if component_path.is_dir() {
                    Colour::Blue.bold()
                } else {
                    // Loop up file extension
                    if let Some(ref ext_styles) = config.extension_styles {
                        component_path.extension()
                                      .and_then(|e| e.to_str())
                                      .and_then(|e| ext_styles.get(e))
                                      .map(|r| r.clone())
                                      .unwrap_or(Colour::White.normal())
                    }
                    else {
                        Colour::White.normal()
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
                  .map(|_| print_entry(&root, path_rel, &config));
    }
}

/// Print error message to stderr and exit with status `1`.
fn error(message: &str) -> ! {
    writeln!(&mut std::io::stderr(), "{}", message)
        .expect("Failed writing to stderr");
    process::exit(1);
}

/// Parse `dircolors` file.
fn parse_dircolors(path: &Path) -> std::io::Result<ExtensionStyles> {
    let file = File::open(path)?;
    let mut ext_styles = HashMap::new();

    let pattern =
        Regex::new(r"^\.([A-Za-z0-9]+)\s*38;5;([0-9]+)\b").unwrap();

    for line in BufReader::new(file).lines() {
        if let Some(caps) = pattern.captures(line.unwrap().as_str()) {
            if let Some(ext) = caps.get(1).map(|m| m.as_str()) {
                let fg = caps.get(2)
                             .map(|m| m.as_str())
                             .and_then(|n| u8::from_str_radix(n, 10).ok())
                             .unwrap_or(7); // white
                ext_styles.insert(String::from(ext),
                                  Colour::Fixed(fg).normal());
            }
        }
    }
    Ok(ext_styles)
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
                     format!("maximum search depth (default: {})",
                             MAX_DEPTH_DEFAULT).as_str(),
                     "D");

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

    let ext_styles = env::home_dir()
                         .map(|h| h.join(".dir_colors"))
                         .and_then(|path| parse_dircolors(&path).ok());

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
                   .unwrap_or(MAX_DEPTH_DEFAULT),
        extension_styles:  ext_styles
    };

    match RegexBuilder::new(pattern)
              .case_insensitive(!config.case_sensitive)
              .build() {
        Ok(re)   => scan(&current_dir, &re, &config),
        Err(err) => error(err.description())
    }
}

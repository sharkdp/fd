use std::error::Error;

use globset;
use regex::RegexBuilder;

use internal::error;

// http://pubs.opengroup.org/onlinepubs/9699919799/functions/glob.html
//
// TODO: Custom rules:
// 1. On Windows, all "\" in the path must be replaced with "/" before matching.
// 2. "\" removes special meaning of any single following character, then be discarded.
// 3. No character class expression?
// 4. Do not skip dot-files.
// 5. Ignore system locales.
//
// TODO: Make a new fork of globset? With a simpler rules set?
pub struct GlobBuilder {}

impl GlobBuilder {
    pub fn new(pattern: &str, search_full_path: bool) -> RegexBuilder {
        #[cfg(windows)]
        let pattern = &patch_glob_pattern(pattern, search_full_path);

        match globset::GlobBuilder::new(pattern)
            .literal_separator(search_full_path)
            .build() {
            Ok(glob) => {
                eprintln!("PATTERN: {} -> {}", glob.glob(), glob.regex());
                // NOTE: .replace("(?-u)", "") works with globset 0.2.0
                // FIXME: do not escape multi-byte chars with \xHH
                RegexBuilder::new(glob.regex().replace("(?-u)", "").as_str())
            }
            Err(err) => error(err.description()),
        }
    }
}

#[cfg(windows)]
fn patch_glob_pattern(pattern: &str, search_full_path: bool) -> String {
    if search_full_path {
        let mut s = String::new();

        if pattern.starts_with("/") {
            s.push_str(&get_default_root());
        } else if pattern.starts_with("*") {
            s.push_str(&get_default_root());
            if cfg!(windows) {
                s.push('/');
            }
        } // else if start with "[/]"? TODO
        s.push_str(pattern);
        s
    } else {
        pattern.to_string()
    }
}

#[cfg(windows)]
fn get_default_root() -> String {
    use std::env;

    if let Ok(cwd) = env::current_dir() {
        let mut compos = cwd.components();
        compos.next().map_or(String::from(""), |compo| {
            // FIXME: escape special chars
            compo.as_os_str().to_string_lossy().into()
        })
    } else {
        error("Error: could not get current directory.");
    }
}

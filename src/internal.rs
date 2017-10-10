use std::process;
use std::time;
use std::io::Write;

use lscolors::LsColors;
use walk::FileType;

/// Root directory
#[cfg(unix)]
pub static ROOT_DIR: &'static str = "/";

#[cfg(windows)]
pub static ROOT_DIR: &'static str = "";

/// Defines how to display search result paths.
#[derive(PartialEq)]
pub enum PathDisplay {
    /// As an absolute path
    Absolute,

    /// As a relative path
    Relative,
}

/// Configuration options for *fd*.
pub struct FdOptions {
    /// Determines whether the regex search is case-sensitive or case-insensitive.
    pub case_sensitive: bool,

    /// Whether to search within the full file path or just the base name (filename or directory
    /// name).
    pub search_full_path: bool,

    /// Whether to ignore hidden files and directories (or not).
    pub ignore_hidden: bool,

    /// Whether to respect VCS ignore files (`.gitignore`, `.ignore`, ..) or not.
    pub read_ignore: bool,

    /// Whether to follow symlinks or not.
    pub follow_links: bool,

    /// Whether elements of output should be separated by a null character
    pub null_separator: bool,

    /// The maximum search depth, or `None` if no maximum search depth should be set.
    ///
    /// A depth of `1` includes all files under the current directory, a depth of `2` also includes
    /// all files under subdirectories of the current directory, etc.
    pub max_depth: Option<usize>,

    /// The number of threads to use.
    pub threads: usize,

    /// Time to buffer results internally before streaming to the console. This is useful to
    /// provide a sorted output, in case the total execution time is shorter than
    /// `max_buffer_time`.
    pub max_buffer_time: Option<time::Duration>,

    /// Display results as relative or absolute path.
    pub path_display: PathDisplay,

    /// `None` if the output should not be colorized. Otherwise, a `LsColors` instance that defines
    /// how to style different filetypes.
    pub ls_colors: Option<LsColors>,

    /// The type of file to search for. All files other than the specified type will be ignored.
    pub file_type: FileType,

    /// The extension to search for. Only entries matching the extension will be included.
    ///
    /// The value (if present) will be a lowercase string without leading dots.
    pub extension: Option<String>,
}

/// Print error message to stderr and exit with status `1`.
pub fn error(message: &str) -> ! {
    writeln!(&mut ::std::io::stderr(), "{}", message).expect("Failed writing to stderr");
    process::exit(1);
}

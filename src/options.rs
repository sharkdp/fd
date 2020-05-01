use std::{path::PathBuf, sync::Arc, time::Duration};

use lscolors::LsColors;
use regex::bytes::RegexSet;

use crate::exec::CommandTemplate;
use crate::filetypes::FileTypes;
#[cfg(unix)]
use crate::filter::OwnerFilter;
use crate::filter::{SizeFilter, TimeFilter};

/// Configuration options for *fd*.
pub struct Options {
    /// Whether the search is case-sensitive or case-insensitive.
    pub case_sensitive: bool,

    /// Whether to search within the full file path or just the base name (filename or directory
    /// name).
    pub search_full_path: bool,

    /// Whether to ignore hidden files and directories (or not).
    pub ignore_hidden: bool,

    /// Whether to respect `.fdignore` files or not.
    pub read_fdignore: bool,

    /// Whether to respect VCS ignore files (`.gitignore`, ..) or not.
    pub read_vcsignore: bool,

    /// Whether to follow symlinks or not.
    pub follow_links: bool,

    /// Whether to limit the search to starting file system or not.
    pub one_file_system: bool,

    /// Whether elements of output should be separated by a null character
    pub null_separator: bool,

    /// The maximum search depth, or `None` if no maximum search depth should be set.
    ///
    /// A depth of `1` includes all files under the current directory, a depth of `2` also includes
    /// all files under subdirectories of the current directory, etc.
    pub max_depth: Option<usize>,

    /// The minimum depth for reported entries, or `None`.
    pub min_depth: Option<usize>,

    /// The number of threads to use.
    pub threads: usize,

    /// Time to buffer results internally before streaming to the console. This is useful to
    /// provide a sorted output, in case the total execution time is shorter than
    /// `max_buffer_time`.
    pub max_buffer_time: Option<Duration>,

    /// `None` if the output should not be colorized. Otherwise, a `LsColors` instance that defines
    /// how to style different filetypes.
    pub ls_colors: Option<LsColors>,

    /// Whether or not we are writing to an interactive terminal
    pub interactive_terminal: bool,

    /// The type of file to search for. If set to `None`, all file types are displayed. If
    /// set to `Some(..)`, only the types that are specified are shown.
    pub file_types: Option<FileTypes>,

    /// The extension to search for. Only entries matching the extension will be included.
    ///
    /// The value (if present) will be a lowercase string without leading dots.
    pub extensions: Option<RegexSet>,

    /// If a value is supplied, each item found will be used to generate and execute commands.
    pub command: Option<Arc<CommandTemplate>>,

    /// A list of glob patterns that should be excluded from the search.
    pub exclude_patterns: Vec<String>,

    /// A list of custom ignore files.
    pub ignore_files: Vec<PathBuf>,

    /// The given constraints on the size of returned files
    pub size_constraints: Vec<SizeFilter>,

    /// Constraints on last modification time of files
    pub time_constraints: Vec<TimeFilter>,

    #[cfg(unix)]
    /// User/group ownership constraint
    pub owner_constraint: Option<OwnerFilter>,

    /// Whether or not to display filesystem errors
    pub show_filesystem_errors: bool,

    /// The separator used to print file paths.
    pub path_separator: Option<String>,

    /// The maximum number of search results
    pub max_results: Option<usize>,
}

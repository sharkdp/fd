use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::anyhow;
use clap::{
    error::ErrorKind, value_parser, Arg, ArgAction, ArgGroup, ArgMatches, Command, Parser,
    ValueEnum,
};
#[cfg(feature = "completions")]
use clap_complete::Shell;
use normpath::PathExt;

use crate::error::print_error;
use crate::exec::CommandSet;
use crate::filesystem;
#[cfg(unix)]
use crate::filter::OwnerFilter;
use crate::filter::SizeFilter;

#[derive(Parser)]
#[command(
    name = "fd",
    version,
    about = "A program to find entries in your filesystem",
    after_long_help = "Bugs can be reported on GitHub: https://github.com/sharkdp/fd/issues",
    max_term_width = 98,
    args_override_self = true,
    group(ArgGroup::new("execs").args(&["exec", "exec_batch", "list_details"]).conflicts_with_all(&[
            "max_results", "quiet", "max_one_result"])),
)]
pub struct Opts {
    /// Include hidden directories and files in the search results (default:
    /// hidden files and directories are skipped). Files and directories are
    /// considered to be hidden if their name starts with a `.` sign (dot).
    /// Any files or directories that are ignored due to the rules described by
    /// --no-ignore are still ignored unless otherwise specified.
    /// The flag can be overridden with --no-hidden.
    #[arg(
        long,
        short = 'H',
        help = "Search hidden files and directories",
        long_help
    )]
    pub hidden: bool,

    /// Overrides --hidden
    #[arg(long, overrides_with = "hidden", hide = true, action = ArgAction::SetTrue)]
    no_hidden: (),

    /// Show search results from files and directories that would otherwise be
    /// ignored by '.gitignore', '.ignore', '.fdignore', or the global ignore file,
    /// The flag can be overridden with --ignore.
    #[arg(
        long,
        short = 'I',
        help = "Do not respect .(git|fd)ignore files",
        long_help
    )]
    pub no_ignore: bool,

    /// Overrides --no-ignore
    #[arg(long, overrides_with = "no_ignore", hide = true, action = ArgAction::SetTrue)]
    ignore: (),

    ///Show search results from files and directories that
    ///would otherwise be ignored by '.gitignore' files.
    ///The flag can be overridden with --ignore-vcs.
    #[arg(
        long,
        hide_short_help = true,
        help = "Do not respect .gitignore files",
        long_help
    )]
    pub no_ignore_vcs: bool,

    /// Overrides --no-ignore-vcs
    #[arg(long, overrides_with = "no_ignore_vcs", hide = true, action = ArgAction::SetTrue)]
    ignore_vcs: (),

    /// Do not require a git repository to respect gitignores.
    /// By default, fd will only respect global gitignore rules, .gitignore rules,
    /// and local exclude rules if fd detects that you are searching inside a
    /// git repository. This flag allows you to relax this restriction such that
    /// fd will respect all git related ignore rules regardless of whether you're
    /// searching in a git repository or not.
    ///
    ///
    /// This flag can be disabled with --require-git.
    #[arg(
        long,
        overrides_with = "require_git",
        hide_short_help = true,
        // same description as ripgrep's flag: ripgrep/crates/core/app.rs
        long_help
    )]
    pub no_require_git: bool,

    /// Overrides --no-require-git
    #[arg(long, overrides_with = "no_require_git", hide = true, action = ArgAction::SetTrue)]
    require_git: (),

    /// Show search results from files and directories that would otherwise be
    /// ignored by '.gitignore', '.ignore', or '.fdignore' files in parent directories.
    #[arg(
        long,
        hide_short_help = true,
        help = "Do not respect .(git|fd)ignore files in parent directories",
        long_help
    )]
    pub no_ignore_parent: bool,

    /// Do not respect the global ignore file
    #[arg(long, hide = true)]
    pub no_global_ignore_file: bool,

    /// Perform an unrestricted search, including ignored and hidden files. This is
    /// an alias for '--no-ignore --hidden'.
    #[arg(long = "unrestricted", short = 'u', overrides_with_all(&["ignore", "no_hidden"]), action(ArgAction::Count), hide_short_help = true,
    help = "Unrestricted search, alias for '--no-ignore --hidden'",
        long_help,
        )]
    rg_alias_hidden_ignore: u8,

    /// Case-sensitive search (default: smart case)
    #[arg(
        long,
        short = 's',
        overrides_with("ignore_case"),
        long_help = "Perform a case-sensitive search. By default, fd uses case-insensitive \
                     searches, unless the pattern contains an uppercase character (smart \
                     case)."
    )]
    pub case_sensitive: bool,

    /// Perform a case-insensitive search. By default, fd uses case-insensitive
    /// searches, unless the pattern contains an uppercase character (smart
    /// case).
    #[arg(
        long,
        short = 'i',
        overrides_with("case_sensitive"),
        help = "Case-insensitive search (default: smart case)",
        long_help
    )]
    pub ignore_case: bool,

    /// Perform a glob-based search instead of a regular expression search.
    #[arg(
        long,
        short = 'g',
        conflicts_with("fixed_strings"),
        help = "Glob-based search (default: regular expression)",
        long_help
    )]
    pub glob: bool,

    /// Perform a regular-expression based search (default). This can be used to
    /// override --glob.
    #[arg(
        long,
        overrides_with("glob"),
        hide_short_help = true,
        help = "Regular-expression based search (default)",
        long_help
    )]
    pub regex: bool,

    /// Treat the pattern as a literal string instead of a regular expression. Note
    /// that this also performs substring comparison. If you want to match on an
    /// exact filename, consider using '--glob'.
    #[arg(
        long,
        short = 'F',
        alias = "literal",
        hide_short_help = true,
        help = "Treat pattern as literal string stead of regex",
        long_help
    )]
    pub fixed_strings: bool,

    /// Add additional required search patterns, all of which must be matched. Multiple
    /// additional patterns can be specified. The patterns are regular
    /// expressions, unless '--glob' or '--fixed-strings' is used.
    #[arg(
        long = "and",
        value_name = "pattern",
        help = "Additional search patterns that need to be matched",
        long_help,
        hide_short_help = true,
        allow_hyphen_values = true
    )]
    pub exprs: Option<Vec<String>>,

    /// Shows the full path starting from the root as opposed to relative paths.
    /// The flag can be overridden with --relative-path.
    #[arg(
        long,
        short = 'a',
        help = "Show absolute instead of relative paths",
        long_help
    )]
    pub absolute_path: bool,

    /// Overrides --absolute-path
    #[arg(long, overrides_with = "absolute_path", hide = true, action = ArgAction::SetTrue)]
    relative_path: (),

    /// Use a detailed listing format like 'ls -l'. This is basically an alias
    /// for '--exec-batch ls -l' with some additional 'ls' options. This can be
    /// used to see more metadata, to show symlink targets and to achieve a
    /// deterministic sort order.
    #[arg(
        long,
        short = 'l',
        conflicts_with("absolute_path"),
        help = "Use a long listing format with file metadata",
        long_help
    )]
    pub list_details: bool,

    /// Follow symbolic links
    #[arg(
        long,
        short = 'L',
        alias = "dereference",
        long_help = "By default, fd does not descend into symlinked directories. Using this \
                     flag, symbolic links are also traversed. \
                     Flag can be overridden with --no-follow."
    )]
    pub follow: bool,

    /// Overrides --follow
    #[arg(long, overrides_with = "follow", hide = true, action = ArgAction::SetTrue)]
    no_follow: (),

    /// By default, the search pattern is only matched against the filename (or directory name). Using this flag, the pattern is matched against the full (absolute) path. Example:
    ///   fd --glob -p '**/.git/config'
    #[arg(
        long,
        short = 'p',
        help = "Search full abs. path (default: filename only)",
        long_help,
        verbatim_doc_comment
    )]
    pub full_path: bool,

    /// Separate search results by the null character (instead of newlines).
    /// Useful for piping results to 'xargs'.
    #[arg(
        long = "print0",
        short = '0',
        conflicts_with("list_details"),
        hide_short_help = true,
        help = "Separate search results by the null character",
        long_help
    )]
    pub null_separator: bool,

    /// Limit the directory traversal to a given depth. By default, there is no
    /// limit on the search depth.
    #[arg(
        long,
        short = 'd',
        value_name = "depth",
        alias("maxdepth"),
        help = "Set maximum search depth (default: none)",
        long_help
    )]
    max_depth: Option<usize>,

    /// Only show search results starting at the given depth.
    /// See also: '--max-depth' and '--exact-depth'
    #[arg(
        long,
        value_name = "depth",
        hide_short_help = true,
        alias("mindepth"),
        help = "Only show search results starting at the given depth.",
        long_help
    )]
    min_depth: Option<usize>,

    /// Only show search results at the exact given depth. This is an alias for
    /// '--min-depth <depth> --max-depth <depth>'.
    #[arg(long, value_name = "depth", hide_short_help = true, conflicts_with_all(&["max_depth", "min_depth"]),
    help = "Only show search results at the exact given depth",
        long_help,
        )]
    exact_depth: Option<usize>,

    /// Exclude files/directories that match the given glob pattern. This
    /// overrides any other ignore logic. Multiple exclude patterns can be
    /// specified.
    ///
    /// Examples:
    /// {n}  --exclude '*.pyc'
    /// {n}  --exclude node_modules
    #[arg(
        long,
        short = 'E',
        value_name = "pattern",
        help = "Exclude entries that match the given glob pattern",
        long_help
    )]
    pub exclude: Vec<String>,

    /// Do not traverse into directories that match the search criteria. If
    /// you want to exclude specific directories, use the '--exclude=…' option.
    #[arg(long, hide_short_help = true, conflicts_with_all(&["size", "exact_depth"]),
        long_help,
        )]
    pub prune: bool,

    /// Filter the search by type:
    /// {n}  'f' or 'file':         regular files
    /// {n}  'd' or 'dir' or 'directory':    directories
    /// {n}  'l' or 'symlink':      symbolic links
    /// {n}  's' or 'socket':       socket
    /// {n}  'p' or 'pipe':         named pipe (FIFO)
    /// {n}  'b' or 'block-device': block device
    /// {n}  'c' or 'char-device':  character device
    /// {n}{n}  'x' or 'executable':   executables
    /// {n}  'e' or 'empty':        empty files or directories
    ///
    /// This option can be specified more than once to include multiple file types.
    /// Searching for '--type file --type symlink' will show both regular files as
    /// well as symlinks. Note that the 'executable' and 'empty' filters work differently:
    /// '--type executable' implies '--type file' by default. And '--type empty' searches
    /// for empty files and directories, unless either '--type file' or '--type directory'
    /// is specified in addition.
    ///
    /// Examples:
    /// {n}  - Only search for files:
    /// {n}      fd --type file …
    /// {n}      fd -tf …
    /// {n}  - Find both files and symlinks
    /// {n}      fd --type file --type symlink …
    /// {n}      fd -tf -tl …
    /// {n}  - Find executable files:
    /// {n}      fd --type executable
    /// {n}      fd -tx
    /// {n}  - Find empty files:
    /// {n}      fd --type empty --type file
    /// {n}      fd -te -tf
    /// {n}  - Find empty directories:
    /// {n}      fd --type empty --type directory
    /// {n}      fd -te -td
    #[arg(
        long = "type",
        short = 't',
        value_name = "filetype",
        hide_possible_values = true,
        value_enum,
        help = "Filter by type: file (f), directory (d/dir), symlink (l), \
                executable (x), empty (e), socket (s), pipe (p), \
                char-device (c), block-device (b)",
        long_help
    )]
    pub filetype: Option<Vec<FileType>>,

    /// (Additionally) filter search results by their file extension. Multiple
    /// allowable file extensions can be specified.
    ///
    /// If you want to search for files without extension,
    /// you can use the regex '^[^.]+$' as a normal search pattern.
    #[arg(
        long = "extension",
        short = 'e',
        value_name = "ext",
        help = "Filter by file extension",
        long_help
    )]
    pub extensions: Option<Vec<String>>,

    /// Limit results based on the size of files using the format <+-><NUM><UNIT>.
    ///    '+': file size must be greater than or equal to this
    ///    '-': file size must be less than or equal to this
    ///
    /// If neither '+' nor '-' is specified, file size must be exactly equal to this.
    ///    'NUM':  The numeric size (e.g. 500)
    ///    'UNIT': The units for NUM. They are not case-sensitive.
    /// Allowed unit values:
    ///     'b':  bytes
    ///     'k':  kilobytes (base ten, 10^3 = 1000 bytes)
    ///     'm':  megabytes
    ///     'g':  gigabytes
    ///     't':  terabytes
    ///     'ki': kibibytes (base two, 2^10 = 1024 bytes)
    ///     'mi': mebibytes
    ///     'gi': gibibytes
    ///     'ti': tebibytes
    #[arg(long, short = 'S', value_parser = SizeFilter::from_string, allow_hyphen_values = true, verbatim_doc_comment, value_name = "size",
        help = "Limit results based on the size of files",
        long_help,
        verbatim_doc_comment,
        )]
    pub size: Vec<SizeFilter>,

    /// Filter results based on the file modification time. Files with modification times
    /// greater than the argument are returned. The argument can be provided
    /// as a specific point in time (YYYY-MM-DD HH:MM:SS or @timestamp) or as a duration (10h, 1d, 35min).
    /// If the time is not specified, it defaults to 00:00:00.
    /// '--change-newer-than', '--newer', or '--changed-after' can be used as aliases.
    ///
    /// Examples:
    /// {n}    --changed-within 2weeks
    /// {n}    --change-newer-than '2018-10-27 10:00:00'
    /// {n}    --newer 2018-10-27
    /// {n}    --changed-after 1day
    #[arg(
        long,
        alias("change-newer-than"),
        alias("newer"),
        alias("changed-after"),
        value_name = "date|dur",
        help = "Filter by file modification time (newer than)",
        long_help
    )]
    pub changed_within: Option<String>,

    /// Filter results based on the file modification time. Files with modification times
    /// less than the argument are returned. The argument can be provided
    /// as a specific point in time (YYYY-MM-DD HH:MM:SS or @timestamp) or as a duration (10h, 1d, 35min).
    /// '--change-older-than' or '--older' can be used as aliases.
    ///
    /// Examples:
    /// {n}    --changed-before '2018-10-27 10:00:00'
    /// {n}    --change-older-than 2weeks
    /// {n}    --older 2018-10-27
    #[arg(
        long,
        alias("change-older-than"),
        alias("older"),
        value_name = "date|dur",
        help = "Filter by file modification time (older than)",
        long_help
    )]
    pub changed_before: Option<String>,

    /// Filter files by their user and/or group.
    /// Format: [(user|uid)][:(group|gid)]. Either side is optional.
    /// Precede either side with a '!' to exclude files instead.
    ///
    /// Examples:
    /// {n}    --owner john
    /// {n}    --owner :students
    /// {n}    --owner '!john:students'
    #[cfg(unix)]
    #[arg(long, short = 'o', value_parser = OwnerFilter::from_string, value_name = "user:group",
        help = "Filter by owning user and/or group",
        long_help,
        )]
    pub owner: Option<OwnerFilter>,

    /// Instead of printing the file normally, print the format string with the following placeholders replaced:
    ///   '{}': path (of the current search result)
    ///   '{/}': basename
    ///   '{//}': parent directory
    ///   '{.}': path without file extension
    ///   '{/.}': basename without file extension
    #[arg(
        long,
        value_name = "fmt",
        help = "Print results according to template",
        conflicts_with = "list_details"
    )]
    pub format: Option<String>,

    #[command(flatten)]
    pub exec: Exec,

    /// Maximum number of arguments to pass to the command given with -X.
    /// If the number of results is greater than the given size,
    /// the command given with -X is run again with remaining arguments.
    /// A batch size of zero means there is no limit (default), but note
    /// that batching might still happen due to OS restrictions on the
    /// maximum length of command lines.
    #[arg(
        long,
        value_name = "size",
        hide_short_help = true,
        requires("exec_batch"),
        value_parser = value_parser!(usize),
        default_value_t,
        help = "Max number of arguments to run as a batch size with -X",
        long_help,
    )]
    pub batch_size: usize,

    /// Add a custom ignore-file in '.gitignore' format. These files have a low precedence.
    #[arg(
        long,
        value_name = "path",
        hide_short_help = true,
        help = "Add a custom ignore-file in '.gitignore' format",
        long_help
    )]
    pub ignore_file: Vec<PathBuf>,

    /// Use a custom file name for ignore files. When this is set, fd will not
    /// look for '.ignore' or '.fdignore' files, but for a file with the given
    /// name in each directory. If the custom ignore file is empty, all entries
    /// in its directory are ignored.
    #[arg(
        long,
        value_name = "name",
        hide_short_help = true,
        help = "Use a custom name for ignore files instead of .ignore/.fdignore",
        long_help
    )]
    pub ignore_file_name: Option<String>,

    /// Declare when to use color for the pattern match output
    #[arg(
        long,
        short = 'c',
        value_enum,
        default_value_t = ColorWhen::Auto,
        value_name = "when",
        help = "When to use colors",
        long_help,
    )]
    pub color: ColorWhen,

    /// Add a terminal hyperlink to a file:// url for each path in the output.
    ///
    /// Auto mode  is used if no argument is given to this option.
    ///
    /// This doesn't do anything for --exec and --exec-batch.
    #[arg(
        long,
        alias = "hyper",
        value_name = "when",
        require_equals = true,
        value_enum,
        default_value_t = HyperlinkWhen::Never,
        default_missing_value = "auto",
        num_args = 0..=1,
        help = "Add hyperlinks to output paths"
    )]
    pub hyperlink: HyperlinkWhen,

    /// Set number of threads to use for searching & executing (default: number
    /// of available CPU cores)
    #[arg(long, short = 'j', value_name = "num", hide_short_help = true, value_parser = str::parse::<NonZeroUsize>)]
    pub threads: Option<NonZeroUsize>,

    /// Milliseconds to buffer before streaming search results to console
    ///
    /// Amount of time in milliseconds to buffer, before streaming the search
    /// results to the console.
    #[arg(long, hide = true, value_parser = parse_millis)]
    pub max_buffer_time: Option<Duration>,

    ///Limit the number of search results to 'count' and quit immediately.
    #[arg(
        long,
        value_name = "count",
        hide_short_help = true,
        overrides_with("max_one_result"),
        help = "Limit the number of search results",
        long_help
    )]
    max_results: Option<usize>,

    /// Limit the search to a single result and quit immediately.
    /// This is an alias for '--max-results=1'.
    #[arg(
        short = '1',
        hide_short_help = true,
        overrides_with("max_results"),
        help = "Limit search to a single result",
        long_help
    )]
    max_one_result: bool,

    /// When the flag is present, the program does not print anything and will
    /// return with an exit code of 0 if there is at least one match. Otherwise, the
    /// exit code will be 1.
    /// '--has-results' can be used as an alias.
    #[arg(
        long,
        short = 'q',
        alias = "has-results",
        hide_short_help = true,
        conflicts_with("max_results"),
        help = "Print nothing, exit code 0 if match found, 1 otherwise",
        long_help
    )]
    pub quiet: bool,

    /// Enable the display of filesystem errors for situations such as
    /// insufficient permissions or dead symlinks.
    #[arg(
        long,
        hide_short_help = true,
        help = "Show filesystem errors",
        long_help
    )]
    pub show_errors: bool,

    /// Change the current working directory of fd to the provided path. This
    /// means that search results will be shown with respect to the given base
    /// path. Note that relative paths which are passed to fd via the positional
    /// <path> argument or the '--search-path' option will also be resolved
    /// relative to this directory.
    #[arg(
        long,
        short = 'C',
        value_name = "path",
        hide_short_help = true,
        help = "Change current working directory",
        long_help
    )]
    pub base_directory: Option<PathBuf>,

    /// the search pattern which is either a regular expression (default) or a glob
    /// pattern (if --glob is used). If no pattern has been specified, every entry
    /// is considered a match. If your pattern starts with a dash (-), make sure to
    /// pass '--' first, or it will be considered as a flag (fd -- '-foo').
    #[arg(
        default_value = "",
        hide_default_value = true,
        value_name = "pattern",
        help = "the search pattern (a regular expression, unless '--glob' is used; optional)",
        long_help
    )]
    pub pattern: String,

    /// Set the path separator to use when printing file paths. The default is
    /// the OS-specific separator ('/' on Unix, '\' on Windows).
    #[arg(
        long,
        value_name = "separator",
        hide_short_help = true,
        help = "Set path separator when printing file paths",
        long_help
    )]
    pub path_separator: Option<String>,

    /// The directory where the filesystem search is rooted (optional). If
    /// omitted, search the current working directory.
    #[arg(action = ArgAction::Append,
        value_name = "path",
        help = "the root directories for the filesystem search (optional)",
        long_help,
        )]
    path: Vec<PathBuf>,

    /// Provide paths to search as an alternative to the positional <path>
    /// argument. Changes the usage to `fd [OPTIONS] --search-path <path>
    /// --search-path <path2> [<pattern>]`
    #[arg(
        long,
        conflicts_with("path"),
        value_name = "search-path",
        hide_short_help = true,
        help = "Provides paths to search as an alternative to the positional <path> argument",
        long_help
    )]
    search_path: Vec<PathBuf>,

    /// By default, relative paths are prefixed with './' when -x/--exec,
    /// -X/--exec-batch, or -0/--print0 are given, to reduce the risk of a
    /// path starting with '-' being treated as a command line option. Use
    /// this flag to change this behavior. If this flag is used without a value,
    /// it is equivalent to passing "always".
    #[arg(long, conflicts_with_all(&["path", "search_path"]), value_name = "when", hide_short_help = true, require_equals = true, long_help)]
    strip_cwd_prefix: Option<Option<StripCwdWhen>>,

    /// By default, fd will traverse the file system tree as far as other options
    /// dictate. With this flag, fd ensures that it does not descend into a
    /// different file system than the one it started in. Comparable to the -mount
    /// or -xdev filters of find(1).
    #[cfg(any(unix, windows))]
    #[arg(long, aliases(&["mount", "xdev"]), hide_short_help = true, long_help)]
    pub one_file_system: bool,

    #[cfg(feature = "completions")]
    #[arg(long, hide = true, exclusive = true)]
    gen_completions: Option<Option<Shell>>,
}

impl Opts {
    pub fn search_paths(&self) -> anyhow::Result<Vec<PathBuf>> {
        // would it make sense to concatenate these?
        let paths = if !self.path.is_empty() {
            &self.path
        } else if !self.search_path.is_empty() {
            &self.search_path
        } else {
            let current_directory = Path::new("./");
            ensure_current_directory_exists(current_directory)?;
            return Ok(vec![self.normalize_path(current_directory)]);
        };
        Ok(paths
            .iter()
            .filter_map(|path| {
                if filesystem::is_existing_directory(path) {
                    Some(self.normalize_path(path))
                } else {
                    print_error(format!(
                        "Search path '{}' is not a directory.",
                        path.to_string_lossy()
                    ));
                    None
                }
            })
            .collect())
    }

    fn normalize_path(&self, path: &Path) -> PathBuf {
        if self.absolute_path {
            filesystem::absolute_path(path.normalize().unwrap().as_path()).unwrap()
        } else if path == Path::new(".") {
            // Change "." to "./" as a workaround for https://github.com/BurntSushi/ripgrep/pull/2711
            PathBuf::from("./")
        } else {
            path.to_path_buf()
        }
    }

    pub fn no_search_paths(&self) -> bool {
        self.path.is_empty() && self.search_path.is_empty()
    }

    #[inline]
    pub fn rg_alias_ignore(&self) -> bool {
        self.rg_alias_hidden_ignore > 0
    }

    pub fn max_depth(&self) -> Option<usize> {
        self.max_depth.or(self.exact_depth)
    }

    pub fn min_depth(&self) -> Option<usize> {
        self.min_depth.or(self.exact_depth)
    }

    pub fn threads(&self) -> NonZeroUsize {
        self.threads.unwrap_or_else(default_num_threads)
    }

    pub fn max_results(&self) -> Option<usize> {
        self.max_results
            .filter(|&m| m > 0)
            .or_else(|| self.max_one_result.then_some(1))
    }

    pub fn strip_cwd_prefix<P: FnOnce() -> bool>(&self, auto_pred: P) -> bool {
        use self::StripCwdWhen::*;
        self.no_search_paths()
            && match self.strip_cwd_prefix.map_or(Auto, |o| o.unwrap_or(Always)) {
                Auto => auto_pred(),
                Always => true,
                Never => false,
            }
    }

    #[cfg(feature = "completions")]
    pub fn gen_completions(&self) -> anyhow::Result<Option<Shell>> {
        self.gen_completions
            .map(|maybe_shell| match maybe_shell {
                Some(sh) => Ok(sh),
                None => {
                    Shell::from_env().ok_or_else(|| anyhow!("Unable to get shell from environment"))
                }
            })
            .transpose()
    }
}

/// Get the default number of threads to use, if not explicitly specified.
fn default_num_threads() -> NonZeroUsize {
    // If we can't get the amount of parallelism for some reason, then
    // default to a single thread, because that is safe.
    let fallback = NonZeroUsize::MIN;
    // To limit startup overhead on massively parallel machines, don't use more
    // than 64 threads.
    let limit = NonZeroUsize::new(64).unwrap();

    std::thread::available_parallelism()
        .unwrap_or(fallback)
        .min(limit)
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum FileType {
    #[value(alias = "f")]
    File,
    #[value(alias = "d", alias = "dir")]
    Directory,
    #[value(alias = "l")]
    Symlink,
    #[value(alias = "b")]
    BlockDevice,
    #[value(alias = "c")]
    CharDevice,
    /// A file which is executable by the current effective user
    #[value(alias = "x")]
    Executable,
    #[value(alias = "e")]
    Empty,
    #[value(alias = "s")]
    Socket,
    #[value(alias = "p")]
    Pipe,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
pub enum ColorWhen {
    /// show colors if the output goes to an interactive console (default)
    Auto,
    /// always use colorized output
    Always,
    /// do not use colorized output
    Never,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
pub enum StripCwdWhen {
    /// Use the default behavior
    Auto,
    /// Always strip the ./ at the beginning of paths
    Always,
    /// Never strip the ./
    Never,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
pub enum HyperlinkWhen {
    /// Use hyperlinks only if color is enabled
    Auto,
    /// Always use hyperlinks when printing file paths
    Always,
    /// Never use hyperlinks
    Never,
}

// there isn't a derive api for getting grouped values yet,
// so we have to use hand-rolled parsing for exec and exec-batch
pub struct Exec {
    pub command: Option<CommandSet>,
}

impl clap::FromArgMatches for Exec {
    fn from_arg_matches(matches: &ArgMatches) -> clap::error::Result<Self> {
        let command = matches
            .get_occurrences::<String>("exec")
            .map(CommandSet::new)
            .or_else(|| {
                matches
                    .get_occurrences::<String>("exec_batch")
                    .map(CommandSet::new_batch)
            })
            .transpose()
            .map_err(|e| clap::Error::raw(ErrorKind::InvalidValue, e))?;
        Ok(Exec { command })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> clap::error::Result<()> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

impl clap::Args for Exec {
    fn augment_args(cmd: Command) -> Command {
        cmd.arg(Arg::new("exec")
            .action(ArgAction::Append)
            .long("exec")
            .short('x')
            .num_args(1..)
                .allow_hyphen_values(true)
                .value_terminator(";")
                .value_name("cmd")
                .conflicts_with("list_details")
                .help("Execute a command for each search result")
                .long_help(
                    "Execute a command for each search result in parallel (use --threads=1 for sequential command execution). \
                     There is no guarantee of the order commands are executed in, and the order should not be depended upon. \
                     All positional arguments following --exec are considered to be arguments to the command - not to fd. \
                     It is therefore recommended to place the '-x'/'--exec' option last.\n\
                     The following placeholders are substituted before the command is executed:\n  \
                       '{}':   path (of the current search result)\n  \
                       '{/}':  basename\n  \
                       '{//}': parent directory\n  \
                       '{.}':  path without file extension\n  \
                       '{/.}': basename without file extension\n  \
                       '{{':   literal '{' (for escaping)\n  \
                       '}}':   literal '}' (for escaping)\n\n\
                     If no placeholder is present, an implicit \"{}\" at the end is assumed.\n\n\
                     Examples:\n\n  \
                       - find all *.zip files and unzip them:\n\n      \
                           fd -e zip -x unzip\n\n  \
                       - find *.h and *.cpp files and run \"clang-format -i ..\" for each of them:\n\n      \
                           fd -e h -e cpp -x clang-format -i\n\n  \
                       - Convert all *.jpg files to *.png files:\n\n      \
                           fd -e jpg -x convert {} {.}.png\
                    ",
                ),
        )
        .arg(
            Arg::new("exec_batch")
                .action(ArgAction::Append)
                .long("exec-batch")
                .short('X')
                .num_args(1..)
                .allow_hyphen_values(true)
                .value_terminator(";")
                .value_name("cmd")
                .conflicts_with_all(["exec", "list_details"])
                .help("Execute a command with all search results at once")
                .long_help(
                    "Execute the given command once, with all search results as arguments.\n\
                     The order of the arguments is non-deterministic, and should not be relied upon.\n\
                     One of the following placeholders is substituted before the command is executed:\n  \
                       '{}':   path (of all search results)\n  \
                       '{/}':  basename\n  \
                       '{//}': parent directory\n  \
                       '{.}':  path without file extension\n  \
                       '{/.}': basename without file extension\n  \
                       '{{':   literal '{' (for escaping)\n  \
                       '}}':   literal '}' (for escaping)\n\n\
                     If no placeholder is present, an implicit \"{}\" at the end is assumed.\n\n\
                     Examples:\n\n  \
                       - Find all test_*.py files and open them in your favorite editor:\n\n      \
                           fd -g 'test_*.py' -X vim\n\n  \
                       - Find all *.rs files and count the lines with \"wc -l ...\":\n\n      \
                           fd -e rs -X wc -l\
                     "
                ),
        )
    }

    fn augment_args_for_update(cmd: Command) -> Command {
        Self::augment_args(cmd)
    }
}

fn parse_millis(arg: &str) -> Result<Duration, std::num::ParseIntError> {
    Ok(Duration::from_millis(arg.parse()?))
}

fn ensure_current_directory_exists(current_directory: &Path) -> anyhow::Result<()> {
    if filesystem::is_existing_directory(current_directory) {
        Ok(())
    } else {
        Err(anyhow!(
            "Could not retrieve current directory (has it been deleted?)."
        ))
    }
}

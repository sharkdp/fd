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

// Type for options that don't have any values, but are used to negate
// earlier options
struct Negations;

impl clap::FromArgMatches for Negations {
    fn from_arg_matches(_: &ArgMatches) -> clap::error::Result<Self> {
        Ok(Negations)
    }

    fn update_from_arg_matches(&mut self, _: &ArgMatches) -> clap::error::Result<()> {
        Ok(())
    }
}

impl clap::Args for Negations {
    fn augment_args(cmd: Command) -> Command {
        Self::augment_args_for_update(cmd)
    }

    fn augment_args_for_update(cmd: Command) -> Command {
        cmd.arg(
            Arg::new("no_hidden")
                .action(ArgAction::Count)
                .long("no-hidden")
                .overrides_with("hidden")
                .hide(true)
                .long_help("Overrides --hidden."),
        )
        .arg(
            Arg::new("ignore")
                .action(ArgAction::Count)
                .long("ignore")
                .overrides_with("no_ignore")
                .hide(true)
                .long_help("Overrides --no-ignore."),
        )
        .arg(
            Arg::new("ignore_vcs")
                .action(ArgAction::Count)
                .long("ignore-vcs")
                .overrides_with("no_ignore_vcs")
                .hide(true)
                .long_help("Overrides --no-ignore-vcs."),
        )
        .arg(
            Arg::new("relative_path")
                .action(ArgAction::Count)
                .long("relative-path")
                .overrides_with("absolute_path")
                .hide(true)
                .long_help("Overrides --absolute-path."),
        )
        .arg(
            Arg::new("no_follow")
                .action(ArgAction::Count)
                .long("no-follow")
                .overrides_with("follow")
                .hide(true)
                .long_help("Overrides --follow."),
        )
    }
}

#[derive(Parser)]
#[command(
    name = "fd",
    version,
    about = "A program to find entries in your filesystem",
    after_long_help = "Bugs can be reported on GitHub: https://github.com/sharkdp/fd/issues",
    max_term_width = 98,
    args_override_self = true,
    group(ArgGroup::new("execs").args(&["exec", "exec_batch", "list_details"]).conflicts_with_all(&[
            "max_results", "has_results", "count"])),
)]
pub struct Opts {
    /// Search hidden files and directories
    #[arg(
        long,
        short = 'H',
        long_help = "Include hidden directories and files in the search results (default: \
                         hidden files and directories are skipped). Files and directories are \
                         considered to be hidden if their name starts with a `.` sign (dot). \
                         The flag can be overridden with --no-hidden."
    )]
    pub hidden: bool,

    /// Do not respect .(git|fd)ignore files
    #[arg(
        long,
        short = 'I',
        long_help = "Show search results from files and directories that would otherwise be \
                         ignored by '.gitignore', '.ignore', '.fdignore', or the global ignore file. \
                         The flag can be overridden with --ignore."
    )]
    pub no_ignore: bool,

    /// Do not respect .gitignore files
    #[arg(
        long,
        hide_short_help = true,
        long_help = "Show search results from files and directories that would otherwise be \
                         ignored by '.gitignore' files. The flag can be overridden with --ignore-vcs."
    )]
    pub no_ignore_vcs: bool,

    /// Do not respect .(git|fd)ignore files in parent directories
    #[arg(
        long,
        hide_short_help = true,
        long_help = "Show search results from files and directories that would otherwise be \
                     ignored by '.gitignore', '.ignore', or '.fdignore' files in parent directories."
    )]
    pub no_ignore_parent: bool,

    /// Do not respect the global ignore file
    #[arg(long, hide = true)]
    pub no_global_ignore_file: bool,

    /// Unrestricted search, alias for '--no-ignore --hidden'
    #[arg(long = "unrestricted", short = 'u', overrides_with_all(&["ignore", "no_hidden"]), action(ArgAction::Count), hide_short_help = true,
    long_help = "Perform an unrestricted search, including ignored and hidden files. This is \
                 an alias for '--no-ignore --hidden'."
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

    /// Case-insensitive search (default: smart case)
    #[arg(
        long,
        short = 'i',
        overrides_with("case_sensitive"),
        long_help = "Perform a case-insensitive search. By default, fd uses case-insensitive \
                     searches, unless the pattern contains an uppercase character (smart \
                     case)."
    )]
    pub ignore_case: bool,

    /// Glob-based search (default: regular expression)
    #[arg(
        long,
        short = 'g',
        conflicts_with("fixed_strings"),
        long_help = "Perform a glob-based search instead of a regular expression search."
    )]
    pub glob: bool,

    /// Regular-expression based search (default)
    #[arg(
        long,
        overrides_with("glob"),
        hide_short_help = true,
        long_help = "Perform a regular-expression based search (default). This can be used to \
                     override --glob."
    )]
    pub regex: bool,

    /// Treat pattern as literal string stead of regex
    #[arg(
        long,
        short = 'F',
        alias = "literal",
        hide_short_help = true,
        long_help = "Treat the pattern as a literal string instead of a regular expression. Note \
                     that this also performs substring comparison. If you want to match on an \
                     exact filename, consider using '--glob'."
    )]
    pub fixed_strings: bool,

    /// Show absolute instead of relative paths
    #[arg(
        long,
        short = 'a',
        long_help = "Shows the full path starting from the root as opposed to relative paths. \
                     The flag can be overridden with --relative-path."
    )]
    pub absolute_path: bool,

    /// Use a long listing format with file metadata
    #[arg(
        long,
        short = 'l',
        conflicts_with("absolute_path"),
        long_help = "Use a detailed listing format like 'ls -l'. This is basically an alias \
                     for '--exec-batch ls -l' with some additional 'ls' options. This can be \
                     used to see more metadata, to show symlink targets and to achieve a \
                     deterministic sort order."
    )]
    pub list_details: bool,

    /// Follow symbolic links
    #[arg(
        long,
        short = 'L',
        alias = "dereference",
        long_help = "By default, fd does not descend into symlinked directories. Using this \
                     flag, symbolic links are also traversed. \
                     Flag can be overriden with --no-follow."
    )]
    pub follow: bool,

    /// Search full abs. path (default: filename only)
    #[arg(
        long,
        short = 'p',
        long_help = "By default, the search pattern is only matched against the filename (or \
                     directory name). Using this flag, the pattern is matched against the full \
                     (absolute) path. Example:\n  \
                       fd --glob -p '**/.git/config'"
    )]
    pub full_path: bool,

    /// Separate search results by the null character
    #[arg(
        long = "print0",
        short = '0',
        conflicts_with("list_details"),
        hide_short_help = true,
        long_help = "Separate search results by the null character (instead of newlines). \
                     Useful for piping results to 'xargs'."
    )]
    pub null_separator: bool,

    /// Set maximum search depth (default: none)
    #[arg(
        long,
        short = 'd',
        value_name = "depth",
        alias("maxdepth"),
        long_help = "Limit the directory traversal to a given depth. By default, there is no \
                     limit on the search depth."
    )]
    max_depth: Option<usize>,

    /// Only show search results starting at the given depth.
    #[arg(
        long,
        value_name = "depth",
        hide_short_help = true,
        long_help = "Only show search results starting at the given depth. \
                     See also: '--max-depth' and '--exact-depth'"
    )]
    min_depth: Option<usize>,

    /// Only show search results at the exact given depth
    #[arg(long, value_name = "depth", hide_short_help = true, conflicts_with_all(&["max_depth", "min_depth"]),
    long_help = "Only show search results at the exact given depth. This is an alias for \
                 '--min-depth <depth> --max-depth <depth>'.",
        )]
    exact_depth: Option<usize>,

    /// Exclude entries that match the given glob pattern
    #[arg(
        long,
        short = 'E',
        value_name = "pattern",
        long_help = "Exclude files/directories that match the given glob pattern. This \
                         overrides any other ignore logic. Multiple exclude patterns can be \
                         specified.\n\n\
                         Examples:\n  \
                           --exclude '*.pyc'\n  \
                           --exclude node_modules"
    )]
    pub exclude: Vec<String>,

    /// Do not traverse into directories that match the search criteria. If
    /// you want to exclude specific directories, use the '--exclude=…' option.
    #[arg(long, hide_short_help = true, conflicts_with_all(&["size", "exact_depth"]),
        long_help = "Do not traverse into directories that match the search criteria. If \
                     you want to exclude specific directories, use the '--exclude=…' option.",
        )]
    pub prune: bool,

    /// Filter by type: file (f), directory (d), symlink (l),
    /// executable (x), empty (e), socket (s), pipe (p)
    #[arg(
        long = "type",
        short = 't',
        value_name = "filetype",
        hide_possible_values = true,
        value_enum,
        long_help = "Filter the search by type:\n  \
                       'f' or 'file':         regular files\n  \
                       'd' or 'directory':    directories\n  \
                       'l' or 'symlink':      symbolic links\n  \
                       's' or 'socket':       socket\n  \
                       'p' or 'pipe':         named pipe (FIFO)\n\n  \
                       'x' or 'executable':   executables\n  \
                       'e' or 'empty':        empty files or directories\n\n\
                     This option can be specified more than once to include multiple file types. \
                     Searching for '--type file --type symlink' will show both regular files as \
                     well as symlinks. Note that the 'executable' and 'empty' filters work differently: \
                     '--type executable' implies '--type file' by default. And '--type empty' searches \
                     for empty files and directories, unless either '--type file' or '--type directory' \
                     is specified in addition.\n\n\
                     Examples:\n  \
                       - Only search for files:\n      \
                           fd --type file …\n      \
                           fd -tf …\n  \
                       - Find both files and symlinks\n      \
                           fd --type file --type symlink …\n      \
                           fd -tf -tl …\n  \
                       - Find executable files:\n      \
                           fd --type executable\n      \
                           fd -tx\n  \
                       - Find empty files:\n      \
                           fd --type empty --type file\n      \
                           fd -te -tf\n  \
                       - Find empty directories:\n      \
                           fd --type empty --type directory\n      \
                           fd -te -td"
    )]
    pub filetype: Option<Vec<FileType>>,

    /// Filter by file extension
    #[arg(
        long = "extension",
        short = 'e',
        value_name = "ext",
        long_help = "(Additionally) filter search results by their file extension. Multiple \
                     allowable file extensions can be specified.\n\
                     If you want to search for files without extension, \
                     you can use the regex '^[^.]+$' as a normal search pattern."
    )]
    pub extensions: Option<Vec<String>>,

    /// Limit results based on the size of files
    #[arg(long, short = 'S', value_parser = SizeFilter::from_string, allow_hyphen_values = true, verbatim_doc_comment, value_name = "size",
        long_help = "Limit results based on the size of files using the format <+-><NUM><UNIT>.\n   \
                        '+': file size must be greater than or equal to this\n   \
                        '-': file size must be less than or equal to this\n\
                     If neither '+' nor '-' is specified, file size must be exactly equal to this.\n   \
                        'NUM':  The numeric size (e.g. 500)\n   \
                        'UNIT': The units for NUM. They are not case-sensitive.\n\
                     Allowed unit values:\n    \
                         'b':  bytes\n    \
                         'k':  kilobytes (base ten, 10^3 = 1000 bytes)\n    \
                         'm':  megabytes\n    \
                         'g':  gigabytes\n    \
                         't':  terabytes\n    \
                         'ki': kibibytes (base two, 2^10 = 1024 bytes)\n    \
                         'mi': mebibytes\n    \
                         'gi': gibibytes\n    \
                         'ti': tebibytes",
        )]
    pub size: Vec<SizeFilter>,

    /// Filter by file modification time (newer than)
    #[arg(
        long,
        alias("change-newer-than"),
        alias("newer"),
        value_name = "date|dur",
        long_help = "Filter results based on the file modification time. The argument can be provided \
                     as a specific point in time (YYYY-MM-DD HH:MM:SS) or as a duration (10h, 1d, 35min). \
                     If the time is not specified, it defaults to 00:00:00. \
                     '--change-newer-than' or '--newer' can be used as aliases.\n\
                     Examples:\n    \
                         --changed-within 2weeks\n    \
                         --change-newer-than '2018-10-27 10:00:00'\n    \
                         --newer 2018-10-27"
    )]
    pub changed_within: Option<String>,

    /// Filter by file modification time (older than)
    #[arg(
        long,
        alias("change-older-than"),
        alias("older"),
        value_name = "date|dur",
        long_help = "Filter results based on the file modification time. The argument can be provided \
                     as a specific point in time (YYYY-MM-DD HH:MM:SS) or as a duration (10h, 1d, 35min). \
                     '--change-older-than' or '--older' can be used as aliases.\n\
                     Examples:\n    \
                         --changed-before '2018-10-27 10:00:00'\n    \
                         --change-older-than 2weeks\n    \
                         --older 2018-10-27"
    )]
    pub changed_before: Option<String>,

    /// Filter by owning user and/or group
    #[cfg(unix)]
    #[arg(long, short = 'o', value_parser = OwnerFilter::from_string, value_name = "user:group",
        long_help = "Filter files by their user and/or group. \
                     Format: [(user|uid)][:(group|gid)]. Either side is optional. \
                     Precede either side with a '!' to exclude files instead.\n\
                     Examples:\n    \
                         --owner john\n    \
                         --owner :students\n    \
                         --owner '!john:students'"
        )]
    pub owner: Option<OwnerFilter>,

    #[command(flatten)]
    pub exec: Exec,

    /// Max number of arguments to run as a batch size with -X
    #[arg(
        long,
        value_name = "size",
        hide_short_help = true,
        requires("exec_batch"),
        value_parser = value_parser!(usize),
        default_value_t,
        long_help = "Maximum number of arguments to pass to the command given with -X. \
                If the number of results is greater than the given size, \
                the command given with -X is run again with remaining arguments. \
                A batch size of zero means there is no limit (default), but note \
                that batching might still happen due to OS restrictions on the \
                maximum length of command lines.",
    )]
    pub batch_size: usize,

    /// Add a custom ignore-file in '.gitignore' format
    #[arg(
        long,
        value_name = "path",
        hide_short_help = true,
        long_help = "Add a custom ignore-file in '.gitignore' format. These files have a low precedence."
    )]
    pub ignore_file: Vec<PathBuf>,

    /// When to use colors
    #[arg(
        long,
        short = 'c',
        value_enum,
        default_value_t = ColorWhen::Auto,
        value_name = "when",
        long_help = "Declare when to use color for the pattern match output",
    )]
    pub color: ColorWhen,

    /// Set number of threads to use for searching & executing (default: number
    /// of available CPU cores)
    #[arg(long, short = 'j', value_name = "num", hide_short_help = true, value_parser = clap::value_parser!(u32).range(1..))]
    pub threads: Option<u32>,

    /// Milliseconds to buffer before streaming search results to console
    ///
    /// Amount of time in milliseconds to buffer, before streaming the search
    /// results to the console.
    #[arg(long, hide = true, value_parser = parse_millis)]
    pub max_buffer_time: Option<Duration>,

    /// Limit number of search results
    #[arg(
        long,
        value_name = "count",
        hide_short_help = true,
        long_help = "Limit the number of search results to 'count' and quit immediately."
    )]
    max_results: Option<usize>,

    /// Limit search to a single result
    #[arg(
        short = '1',
        hide_short_help = true,
        overrides_with("max_results"),
        long_help = "Limit the search to a single result and quit immediately. \
                                This is an alias for '--max-results=1'."
    )]
    max_one_result: bool,

    /// Print nothing, exit code 0 if match found, 1 otherwise
    #[arg(
        long,
        short = 'q',
        alias = "has-results",
        hide_short_help = true,
        conflicts_with("max_results"),
        long_help = "When the flag is present, the program does not print anything and will \
                     return with an exit code of 0 if there is at least one match. Otherwise, the \
                     exit code will be 1. \
                     '--has-results' can be used as an alias."
    )]
    pub quiet: bool,

    /// Show filesystem errors
    #[arg(
        long,
        hide_short_help = true,
        long_help = "Enable the display of filesystem errors for situations such as \
                     insufficient permissions or dead symlinks."
    )]
    pub show_errors: bool,

    /// Change current working directory
    #[arg(
        long,
        value_name = "path",
        hide_short_help = true,
        long_help = "Change the current working directory of fd to the provided path. This \
                         means that search results will be shown with respect to the given base \
                         path. Note that relative paths which are passed to fd via the positional \
                         <path> argument or the '--search-path' option will also be resolved \
                         relative to this directory."
    )]
    pub base_directory: Option<PathBuf>,

    /// the search pattern (a regular expression, unless '--glob' is used; optional)
    #[arg(
        default_value = "",
        hide_default_value = true,
        value_name = "pattern",
        long_help = "the search pattern which is either a regular expression (default) or a glob \
                 pattern (if --glob is used). If no pattern has been specified, every entry \
                 is considered a match. If your pattern starts with a dash (-), make sure to \
                 pass '--' first, or it will be considered as a flag (fd -- '-foo')."
    )]
    pub pattern: String,

    /// Set path separator when printing file paths
    #[arg(
        long,
        value_name = "separator",
        hide_short_help = true,
        long_help = "Set the path separator to use when printing file paths. The default is \
                         the OS-specific separator ('/' on Unix, '\\' on Windows)."
    )]
    pub path_separator: Option<String>,

    /// the root directories for the filesystem search (optional)
    #[arg(action = ArgAction::Append,
        value_name = "path",
        long_help = "The directory where the filesystem search is rooted (optional). If \
                     omitted, search the current working directory.",
        )]
    path: Vec<PathBuf>,

    /// Provides paths to search as an alternative to the positional <path> argument
    #[arg(
        long,
        conflicts_with("path"),
        value_name = "search-path",
        hide_short_help = true,
        long_help = "Provide paths to search as an alternative to the positional <path> \
                     argument. Changes the usage to `fd [OPTIONS] --search-path <path> \
                     --search-path <path2> [<pattern>]`"
    )]
    search_path: Vec<PathBuf>,

    /// By default, relative paths are prefixed with './' when -x/--exec,
    /// -X/--exec-batch, or -0/--print0 are given, to reduce the risk of a
    /// path starting with '-' being treated as a command line option. Use
    /// this flag to disable this behaviour.
    #[arg(long, conflicts_with_all(&["path", "search_path"]), hide_short_help = true,
        long_help = "By default, relative paths are prefixed with './' when -x/--exec, \
    -X/--exec-batch, or -0/--print0 are given, to reduce the risk of a \
    path starting with '-' being treated as a command line option. Use \
    this flag to disable this behaviour.",
    )]
    pub strip_cwd_prefix: bool,

    #[cfg(any(unix, windows))]
    #[arg(long, aliases(&["mount", "xdev"]), hide_short_help = true,
        long_help = "By default, fd will traverse the file system tree as far as other options \
            dictate. With this flag, fd ensures that it does not descend into a \
            different file system than the one it started in. Comparable to the -mount \
            or -xdev filters of find(1).")]
    pub one_file_system: bool,

    #[cfg(feature = "completions")]
    #[arg(long, hide = true, exclusive = true)]
    gen_completions: Option<Option<Shell>>,

    #[clap(flatten)]
    _negations: Negations,
}

impl Opts {
    pub fn search_paths(&self) -> anyhow::Result<Vec<PathBuf>> {
        // would it make sense to concatenate these?
        let paths = if !self.path.is_empty() {
            &self.path
        } else if !self.search_path.is_empty() {
            &self.search_path
        } else {
            let current_directory = Path::new(".");
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

    pub fn threads(&self) -> usize {
        // This will panic if the number of threads passed in is more than usize::MAX in an environment
        // where usize is less than 32 bits (for example 16-bit architectures). It's pretty
        // unlikely fd will be running in such an environment, and even more unlikely someone would
        // be trying to use that many threads on such an environment, so I think panicing is an
        // appropriate way to handle that.
        std::cmp::max(
            self.threads
                .map_or_else(num_cpus::get, |n| n.try_into().expect("too many threads")),
            1,
        )
    }

    pub fn max_results(&self) -> Option<usize> {
        self.max_results
            .filter(|&m| m > 0)
            .or_else(|| self.max_one_result.then(|| 1))
    }

    #[cfg(feature = "completions")]
    pub fn gen_completions(&self) -> anyhow::Result<Option<Shell>> {
        self.gen_completions
            .map(|maybe_shell| match maybe_shell {
                Some(sh) => Ok(sh),
                None => guess_shell(),
            })
            .transpose()
    }
}

#[cfg(feature = "completions")]
fn guess_shell() -> anyhow::Result<Shell> {
    let env_shell = std::env::var_os("SHELL").map(PathBuf::from);
    if let Some(shell) = env_shell
        .as_ref()
        .and_then(|s| s.file_name())
        .and_then(|s| s.to_str())
    {
        shell
            .parse::<Shell>()
            .map_err(|_| anyhow!("Unknown shell {}", shell))
    } else {
        // Assume powershell on windows
        #[cfg(windows)]
        return Ok(Shell::PowerShell);
        #[cfg(not(windows))]
        return Err(anyhow!("Unable to get shell from environment"));
    }
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum FileType {
    #[value(alias = "f")]
    File,
    #[value(alias = "d")]
    Directory,
    #[value(alias = "l")]
    Symlink,
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

impl ColorWhen {
    pub fn as_str(&self) -> &'static str {
        use ColorWhen::*;
        match *self {
            Auto => "auto",
            Never => "never",
            Always => "always",
        }
    }
}

// there isn't a derive api for getting grouped values yet,
// so we have to use hand-rolled parsing for exec and exec-batch
pub struct Exec {
    pub command: Option<CommandSet>,
}

impl clap::FromArgMatches for Exec {
    fn from_arg_matches(matches: &ArgMatches) -> clap::error::Result<Self> {
        let command = matches
            .grouped_values_of("exec")
            .map(CommandSet::new)
            .or_else(|| {
                matches
                    .grouped_values_of("exec_batch")
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
                     All positional arguments following --exec are considered to be arguments to the command - not to fd. \
                     It is therefore recommended to place the '-x'/'--exec' option last.\n\
                     The following placeholders are substituted before the command is executed:\n  \
                       '{}':   path (of the current search result)\n  \
                       '{/}':  basename\n  \
                       '{//}': parent directory\n  \
                       '{.}':  path without file extension\n  \
                       '{/.}': basename without file extension\n\n\
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
                .conflicts_with_all(&["exec", "list_details"])
                .help("Execute a command with all search results at once")
                .long_help(
                    "Execute the given command once, with all search results as arguments.\n\
                     One of the following placeholders is substituted before the command is executed:\n  \
                       '{}':   path (of all search results)\n  \
                       '{/}':  basename\n  \
                       '{//}': parent directory\n  \
                       '{.}':  path without file extension\n  \
                       '{/.}': basename without file extension\n\n\
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

use clap::{crate_version, AppSettings, Arg, ColorChoice, Command};
#[cfg(all(unix, fd_full))]
use crate::filter::OwnerFilter;

#[cfg(not(fd_full))]
mod dummy_parsers {
    macro_rules! dummy_struct_with_parser {
        ($struct_name:ident::$meth_name:ident) => {
            #[derive(Clone)]
            pub(crate) struct $struct_name;

            impl $struct_name {
                pub fn $meth_name(_: &str) -> Result<(), std::convert::Infallible> {
                    Ok(())
                }
            }
        }
    }

    dummy_struct_with_parser!(OwnerFilter::from_string);
}

#[cfg(not(fd_full))]
use dummy_parsers::*;

pub fn build_app() -> Command<'static> {
    let clap_color_choice = if std::env::var_os("NO_COLOR").is_none() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };

    let mut app = Command::new("fd")
        .version(crate_version!())
        .color(clap_color_choice)
        .setting(AppSettings::DeriveDisplayOrder)
        .dont_collapse_args_in_usage(true)
        .after_help(
            "Note: `fd -h` prints a short and concise overview while `fd --help` gives all \
                 details.",
        )
        .arg(
            Arg::new("hidden")
                .long("hidden")
                .short('H')
                .overrides_with("hidden")
                .help("Search hidden files and directories")
                .long_help(
                    "Include hidden directories and files in the search results (default: \
                         hidden files and directories are skipped). Files and directories are \
                         considered to be hidden if their name starts with a `.` sign (dot). \
                         The flag can be overridden with --no-hidden.",
                ),
        )
        .arg(
            Arg::new("no-hidden")
                .long("no-hidden")
                .overrides_with("hidden")
                .hide(true)
                .long_help(
                    "Overrides --hidden.",
                ),
        )
        .arg(
            Arg::new("no-ignore")
                .long("no-ignore")
                .short('I')
                .overrides_with("no-ignore")
                .help("Do not respect .(git|fd)ignore files")
                .long_help(
                    "Show search results from files and directories that would otherwise be \
                         ignored by '.gitignore', '.ignore', '.fdignore', or the global ignore file. \
                         The flag can be overridden with --ignore.",
                ),
        )
        .arg(
            Arg::new("ignore")
                .long("ignore")
                .overrides_with("no-ignore")
                .hide(true)
                .long_help(
                    "Overrides --no-ignore.",
                ),
        )
        .arg(
            Arg::new("no-ignore-vcs")
                .long("no-ignore-vcs")
                .overrides_with("no-ignore-vcs")
                .hide_short_help(true)
                .help("Do not respect .gitignore files")
                .long_help(
                    "Show search results from files and directories that would otherwise be \
                         ignored by '.gitignore' files. The flag can be overridden with --ignore-vcs.",
                ),
        )
        .arg(
            Arg::new("ignore-vcs")
                .long("ignore-vcs")
                .overrides_with("no-ignore-vcs")
                .hide(true)
                .long_help(
                    "Overrides --no-ignore-vcs.",
                ),
        )
        .arg(
            Arg::new("no-ignore-parent")
                .long("no-ignore-parent")
                .overrides_with("no-ignore-parent")
                .hide_short_help(true)
                .help("Do not respect .(git|fd)ignore files in parent directories")
                .long_help(
                    "Show search results from files and directories that would otherwise be \
                        ignored by '.gitignore', '.ignore', or '.fdignore' files in parent directories.",
                ),
        )
        .arg(
            Arg::new("no-global-ignore-file")
                .long("no-global-ignore-file")
                .hide(true)
                .help("Do not respect the global ignore file")
                .long_help("Do not respect the global ignore file."),
        )
        .arg(
            Arg::new("rg-alias-hidden-ignore")
                .short('u')
                .long("unrestricted")
                .overrides_with_all(&["ignore", "no-hidden"])
                .multiple_occurrences(true) // Allowed for historical reasons
                .hide_short_help(true)
                .help("Unrestricted search, alias for '--no-ignore --hidden'")
                .long_help(
                    "Perform an unrestricted search, including ignored and hidden files. This is \
                    an alias for '--no-ignore --hidden'."
                ),
        )
        .arg(
            Arg::new("case-sensitive")
                .long("case-sensitive")
                .short('s')
                .overrides_with_all(&["ignore-case", "case-sensitive"])
                .help("Case-sensitive search (default: smart case)")
                .long_help(
                    "Perform a case-sensitive search. By default, fd uses case-insensitive \
                         searches, unless the pattern contains an uppercase character (smart \
                         case).",
                ),
        )
        .arg(
            Arg::new("ignore-case")
                .long("ignore-case")
                .short('i')
                .overrides_with_all(&["case-sensitive", "ignore-case"])
                .help("Case-insensitive search (default: smart case)")
                .long_help(
                    "Perform a case-insensitive search. By default, fd uses case-insensitive \
                         searches, unless the pattern contains an uppercase character (smart \
                         case).",
                ),
        )
        .arg(
            Arg::new("glob")
                .long("glob")
                .short('g')
                .conflicts_with("fixed-strings")
                .overrides_with("glob")
                .help("Glob-based search (default: regular expression)")
                .long_help("Perform a glob-based search instead of a regular expression search."),
        )
        .arg(
            Arg::new("regex")
                .long("regex")
                .overrides_with_all(&["glob", "regex"])
                .hide_short_help(true)
                .help("Regular-expression based search (default)")
                .long_help(
                    "Perform a regular-expression based search (default). This can be used to \
                         override --glob.",
                ),
        )
        .arg(
            Arg::new("fixed-strings")
                .long("fixed-strings")
                .short('F')
                .alias("literal")
                .overrides_with("fixed-strings")
                .hide_short_help(true)
                .help("Treat pattern as literal string instead of regex")
                .long_help(
                    "Treat the pattern as a literal string instead of a regular expression. Note \
                     that this also performs substring comparison. If you want to match on an \
                     exact filename, consider using '--glob'.",
                ),
        )
        .arg(
            Arg::new("absolute-path")
                .long("absolute-path")
                .short('a')
                .overrides_with("absolute-path")
                .help("Show absolute instead of relative paths")
                .long_help(
                    "Shows the full path starting from the root as opposed to relative paths. \
                     The flag can be overridden with --relative-path.",
                ),
        )
        .arg(
            Arg::new("relative-path")
                .long("relative-path")
                .overrides_with("absolute-path")
                .hide(true)
                .long_help(
                    "Overrides --absolute-path.",
                ),
        )
        .arg(
            Arg::new("list-details")
                .long("list-details")
                .short('l')
                .conflicts_with("absolute-path")
                .help("Use a long listing format with file metadata")
                .long_help(
                    "Use a detailed listing format like 'ls -l'. This is basically an alias \
                         for '--exec-batch ls -l' with some additional 'ls' options. This can be \
                         used to see more metadata, to show symlink targets and to achieve a \
                         deterministic sort order.",
                ),
        )
        .arg(
            Arg::new("follow")
                .long("follow")
                .short('L')
                .alias("dereference")
                .overrides_with("follow")
                .help("Follow symbolic links")
                .long_help(
                    "By default, fd does not descend into symlinked directories. Using this \
                         flag, symbolic links are also traversed. \
                         Flag can be overriden with --no-follow.",
                ),
        )
        .arg(
            Arg::new("no-follow")
                .long("no-follow")
                .overrides_with("follow")
                .hide(true)
                .long_help(
                    "Overrides --follow.",
                ),
        )
        .arg(
            Arg::new("full-path")
                .long("full-path")
                .short('p')
                .overrides_with("full-path")
                .help("Search full abs. path (default: filename only)")
                .long_help(
                    "By default, the search pattern is only matched against the filename (or \
                      directory name). Using this flag, the pattern is matched against the full \
                      (absolute) path. Example:\n  \
                        fd --glob -p '**/.git/config'",
                ),
        )
        .arg(
            Arg::new("null_separator")
                .long("print0")
                .short('0')
                .overrides_with("print0")
                .conflicts_with("list-details")
                .hide_short_help(true)
                .help("Separate results by the null character")
                .long_help(
                    "Separate search results by the null character (instead of newlines). \
                         Useful for piping results to 'xargs'.",
                ),
        )
        .arg(
            Arg::new("max-depth")
                .long("max-depth")
                .short('d')
                .takes_value(true)
                .value_name("depth")
                .help("Set maximum search depth (default: none)")
                .long_help(
                    "Limit the directory traversal to a given depth. By default, there is no \
                         limit on the search depth.",
                ),
        )
        // support --maxdepth as well, for compatibility with rg
        .arg(
            Arg::new("rg-depth")
                .long("maxdepth")
                .hide(true)
                .takes_value(true)
                .help("Set maximum search depth (default: none)")
        )
        .arg(
            Arg::new("min-depth")
                .long("min-depth")
                .takes_value(true)
                .value_name("depth")
                .hide_short_help(true)
                .help("Only show results starting at given depth")
                .long_help(
                    "Only show search results starting at the given depth. \
                     See also: '--max-depth' and '--exact-depth'",
                ),
        )
        .arg(
            Arg::new("exact-depth")
                .long("exact-depth")
                .takes_value(true)
                .value_name("depth")
                .hide_short_help(true)
                .conflicts_with_all(&["max-depth", "min-depth"])
                .help("Only show results at exact given depth")
                .long_help(
                    "Only show search results at the exact given depth. This is an alias for \
                     '--min-depth <depth> --max-depth <depth>'.",
                ),
        )
        .arg(
            Arg::new("prune")
                .long("prune")
                .conflicts_with_all(&["size", "exact-depth"])
                .hide_short_help(true)
                .help("Do not traverse into matching directories")
                .long_help("Do not traverse into directories that match the search criteria. If \
                    you want to exclude specific directories, use the '--exclude=…' option.")
        )
        .arg(
            Arg::new("file-type")
                .long("type")
                .short('t')
                .multiple_occurrences(true)
                .number_of_values(1)
                .takes_value(true)
                .value_name("filetype")
                .possible_values(&[
                    "f",
                    "file",
                    "d",
                    "directory",
                    "l",
                    "symlink",
                    "x",
                    "executable",
                    "e",
                    "empty",
                    "s",
                    "socket",
                    "p",
                    "pipe",
                ])
                .hide_possible_values(true)
                .help(
                    "Filter by type: file (f), directory (d), symlink (l),\nexecutable (x), \
                         empty (e), socket (s), pipe (p)",
                )
                .long_help(
                    "Filter the search by type:\n  \
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
                ),
        )
        .arg(
            Arg::new("extension")
                .long("extension")
                .short('e')
                .multiple_occurrences(true)
                .number_of_values(1)
                .takes_value(true)
                .value_name("ext")
                .help("Filter by file extension")
                .long_help(
                    "(Additionally) filter search results by their file extension. Multiple \
                     allowable file extensions can be specified.\n\
                     If you want to search for files without extension, \
                     you can use the regex '^[^.]+$' as a normal search pattern.",
                ),
        )
        .arg(
            Arg::new("exec")
                .long("exec")
                .short('x')
                .min_values(1)
                .multiple_occurrences(true)
                .allow_hyphen_values(true)
                .value_terminator(";")
                .value_name("cmd")
                .conflicts_with("list-details")
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
            Arg::new("exec-batch")
                .long("exec-batch")
                .short('X')
                .min_values(1)
                .multiple_occurrences(true)
                .allow_hyphen_values(true)
                .value_terminator(";")
                .value_name("cmd")
                .conflicts_with_all(&["exec", "list-details"])
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
        .arg(
            Arg::new("batch-size")
            .long("batch-size")
            .takes_value(true)
            .value_name("size")
            .hide_short_help(true)
            .requires("exec-batch")
            .help("Max number of arguments to run as a batch with -X")
            .long_help(
                "Maximum number of arguments to pass to the command given with -X. \
                If the number of results is greater than the given size, \
                the command given with -X is run again with remaining arguments. \
                A batch size of zero means there is no limit (default), but note \
                that batching might still happen due to OS restrictions on the \
                maximum length of command lines.",
            ),
        )
        .arg(
            Arg::new("exclude")
                .long("exclude")
                .short('E')
                .takes_value(true)
                .value_name("pattern")
                .number_of_values(1)
                .multiple_occurrences(true)
                .help("Exclude entries that match the given glob pattern")
                .long_help(
                    "Exclude files/directories that match the given glob pattern. This \
                         overrides any other ignore logic. Multiple exclude patterns can be \
                         specified.\n\n\
                         Examples:\n  \
                           --exclude '*.pyc'\n  \
                           --exclude node_modules",
                ),
        )
        .arg(
            Arg::new("ignore-file")
                .long("ignore-file")
                .takes_value(true)
                .value_name("path")
                .number_of_values(1)
                .multiple_occurrences(true)
                .hide_short_help(true)
                .help("Add custom ignore-file in '.gitignore' format")
                .long_help(
                    "Add a custom ignore-file in '.gitignore' format. These files have a low \
                         precedence.",
                ),
        )
        .arg(
            Arg::new("color")
                .long("color")
                .short('c')
                .takes_value(true)
                .value_name("when")
                .possible_values(&["never", "auto", "always"])
                .hide_possible_values(true)
                .help("When to use colors: never, *auto*, always")
                .long_help(
                    "Declare when to use color for the pattern match output:\n  \
                       'auto':      show colors if the output goes to an interactive console (default)\n  \
                       'never':     do not use colorized output\n  \
                       'always':    always use colorized output",
                ),
        )
        .arg(
            Arg::new("threads")
                .long("threads")
                .short('j')
                .takes_value(true)
                .value_name("num")
                .hide_short_help(true)
                .help("Set number of threads")
                .long_help(
                    "Set number of threads to use for searching & executing (default: number \
                         of available CPU cores)",
                ),
        )
        .arg(
            Arg::new("size")
                .long("size")
                .short('S')
                .takes_value(true)
                .number_of_values(1)
                .allow_hyphen_values(true)
                .multiple_occurrences(true)
                .help("Limit results based on the size of files")
                .long_help(
                    "Limit results based on the size of files using the format <+-><NUM><UNIT>.\n   \
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
                ),
        )
        .arg(
            Arg::new("max-buffer-time")
                .long("max-buffer-time")
                .takes_value(true)
                .hide(true)
                .help("Milliseconds to buffer before streaming search results to console")
                .long_help(
                    "Amount of time in milliseconds to buffer, before streaming the search \
                         results to the console.",
                ),
        )
        .arg(
            Arg::new("changed-within")
                .long("changed-within")
                .alias("change-newer-than")
                .alias("newer")
                .takes_value(true)
                .value_name("date|dur")
                .number_of_values(1)
                .help("Filter by file modification time (newer than)")
                .long_help(
                    "Filter results based on the file modification time. The argument can be provided \
                     as a specific point in time (YYYY-MM-DD HH:MM:SS) or as a duration (10h, 1d, 35min). \
                     If the time is not specified, it defaults to 00:00:00. \
                     '--change-newer-than' or '--newer' can be used as aliases.\n\
                     Examples:\n    \
                         --changed-within 2weeks\n    \
                         --change-newer-than '2018-10-27 10:00:00'\n    \
                         --newer 2018-10-27",
                ),
        )
        .arg(
            Arg::new("changed-before")
                .long("changed-before")
                .alias("change-older-than")
                .alias("older")
                .takes_value(true)
                .value_name("date|dur")
                .number_of_values(1)
                .help("Filter by file modification time (older than)")
                .long_help(
                    "Filter results based on the file modification time. The argument can be provided \
                     as a specific point in time (YYYY-MM-DD HH:MM:SS) or as a duration (10h, 1d, 35min). \
                     '--change-older-than' or '--older' can be used as aliases.\n\
                     Examples:\n    \
                         --changed-before '2018-10-27 10:00:00'\n    \
                         --change-older-than 2weeks\n    \
                         --older 2018-10-27",
                ),
        )
        .arg(
            Arg::new("max-results")
                .long("max-results")
                .takes_value(true)
                .value_name("count")
                // We currently do not support --max-results in combination with
                // program execution because the results that come up in a --max-results
                // search are non-deterministic. Users might think that they can run the
                // same search with `--exec rm` attached and get a reliable removal of
                // the files they saw in the previous search.
                .conflicts_with_all(&["exec", "exec-batch", "list-details"])
                .hide_short_help(true)
                .help("Limit number of search results")
                .long_help("Limit the number of search results to 'count' and quit immediately."),
        )
        .arg(
            Arg::new("max-one-result")
                .short('1')
                .hide_short_help(true)
                .overrides_with("max-results")
                .conflicts_with_all(&["exec", "exec-batch", "list-details"])
                .help("Limit search to a single result")
                .long_help("Limit the search to a single result and quit immediately. \
                                This is an alias for '--max-results=1'.")
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .short('q')
                .alias("has-results")
                .hide_short_help(true)
                .conflicts_with_all(&["exec", "exec-batch", "list-details", "max-results"])
                .help("Print nothing, exit code 0 if match found, 1 otherwise")
                .long_help(
                    "When the flag is present, the program does not print anything and will \
                     return with an exit code of 0 if there is at least one match. Otherwise, the \
                     exit code will be 1. \
                     '--has-results' can be used as an alias."
                )
        )
        .arg(
            Arg::new("show-errors")
                .long("show-errors")
                .hide_short_help(true)
                .overrides_with("show-errors")
                .help("Show filesystem errors")
                .long_help(
                    "Enable the display of filesystem errors for situations such as \
                         insufficient permissions or dead symlinks.",
                ),
        )
        .arg(
            Arg::new("base-directory")
                .long("base-directory")
                .takes_value(true)
                .value_name("path")
                .number_of_values(1)
                .allow_invalid_utf8(true)
                .hide_short_help(true)
                .help("Change current working directory")
                .long_help(
                    "Change the current working directory of fd to the provided path. This \
                         means that search results will be shown with respect to the given base \
                         path. Note that relative paths which are passed to fd via the positional \
                         <path> argument or the '--search-path' option will also be resolved \
                         relative to this directory.",
                ),
        )
        .arg(
            Arg::new("pattern")
            .allow_invalid_utf8(true)
            .help(
                "the search pattern (a regular expression, unless '--glob' is used; optional)",
            ).long_help(
                "the search pattern which is either a regular expression (default) or a glob \
                 pattern (if --glob is used). If no pattern has been specified, every entry \
                 is considered a match. If your pattern starts with a dash (-), make sure to \
                 pass '--' first, or it will be considered as a flag (fd -- '-foo').")
        )
        .arg(
            Arg::new("path-separator")
                .takes_value(true)
                .value_name("separator")
                .long("path-separator")
                .hide_short_help(true)
                .help("Set path separator when printing file paths")
                .long_help(
                    "Set the path separator to use when printing file paths. The default is \
                         the OS-specific separator ('/' on Unix, '\\' on Windows).",
                ),
        )
        .arg(
            Arg::new("path")
                .multiple_occurrences(true)
                .allow_invalid_utf8(true)
                .help("the root directory for the filesystem search (optional)")
                .long_help(
                    "The directory where the filesystem search is rooted (optional). If \
                         omitted, search the current working directory.",
                ),
        )
        .arg(
            Arg::new("search-path")
                .long("search-path")
                .takes_value(true)
                .conflicts_with("path")
                .multiple_occurrences(true)
                .hide_short_help(true)
                .number_of_values(1)
                .allow_invalid_utf8(true)
                .help("Provide paths to search as an alternative to the positional <path>")
                .long_help(
                    "Provide paths to search as an alternative to the positional <path> \
                         argument. Changes the usage to `fd [OPTIONS] --search-path <path> \
                         --search-path <path2> [<pattern>]`",
                ),
        )
        .arg(
            Arg::new("strip-cwd-prefix")
                .long("strip-cwd-prefix")
                .conflicts_with_all(&["path", "search-path"])
                .hide_short_help(true)
                .help("strip './' prefix from non-tty outputs")
                .long_help(
                    "By default, relative paths are prefixed with './' when the output goes to a non \
                     interactive terminal (TTY). Use this flag to disable this behaviour."
                )
        );

    #[cfg(unix)]
    {
        app = app.arg(
            Arg::new("owner")
                .long("owner")
                .short('o')
                .takes_value(true)
                .value_parser(OwnerFilter::from_string)
                .value_name("user:group")
                .help("Filter by owning user and/or group")
                .long_help(
                    "Filter files by their user and/or group. \
                     Format: [(user|uid)][:(group|gid)]. Either side is optional. \
                     Precede either side with a '!' to exclude files instead.\n\
                     Examples:\n    \
                         --owner john\n    \
                         --owner :students\n    \
                         --owner '!john:students'",
                ),
        );
    }

    // Make `--one-file-system` available only on Unix and Windows platforms, as per the
    // restrictions on the corresponding option in the `ignore` crate.
    // Provide aliases `mount` and `xdev` for people coming from `find`.
    if cfg!(any(unix, windows)) {
        app = app.arg(
            Arg::new("one-file-system")
                .long("one-file-system")
                .aliases(&["mount", "xdev"])
                .hide_short_help(true)
                .help("Do not descend into a different file system")
                .long_help(
                    "By default, fd will traverse the file system tree as far as other options \
                     dictate. With this flag, fd ensures that it does not descend into a \
                     different file system than the one it started in. Comparable to the -mount \
                     or -xdev filters of find(1).",
                ),
        );
    }

    app
}

#[test]
fn verify_app() {
    build_app().debug_assert()
}

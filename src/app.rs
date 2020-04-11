use clap::{crate_version, App, AppSettings, Arg};

pub fn build_app() -> App<'static, 'static> {
    let mut app = App::new("fd")
        .version(crate_version!())
        .usage("fd [FLAGS/OPTIONS] [<pattern>] [<path>...]")
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .after_help(
            "Note: `fd -h` prints a short and concise overview while `fd --help` gives all \
                 details.",
        )
        .arg(
            Arg::with_name("hidden")
                .long("hidden")
                .short("H")
                .overrides_with("hidden")
                .help("Search hidden files and directories")
                .long_help(
                    "Include hidden directories and files in the search results (default: \
                         hidden files and directories are skipped). Files and directories are \
                         considered to be hidden if their name starts with a `.` sign (dot).",
                ),
        )
        .arg(
            Arg::with_name("no-ignore")
                .long("no-ignore")
                .short("I")
                .overrides_with("no-ignore")
                .help("Do not respect .(git|fd)ignore files")
                .long_help(
                    "Show search results from files and directories that would otherwise be \
                         ignored by '.gitignore', '.ignore' or '.fdignore' files.",
                ),
        )
        .arg(
            Arg::with_name("no-ignore-vcs")
                .long("no-ignore-vcs")
                .overrides_with("no-ignore-vcs")
                .help("Do not respect .gitignore files")
                .long_help(
                    "Show search results from files and directories that would otherwise be \
                         ignored by '.gitignore' files.",
                ),
        )
        .arg(
            Arg::with_name("rg-alias-hidden-ignore")
                .short("u")
                .long("unrestricted")
                .multiple(true)
                .hidden_short_help(true)
                .long_help(
                    "Alias for '--no-ignore'. Can be repeated. '-uu' is an alias for \
                         '--no-ignore --hidden'.",
                ),
        )
        .arg(
            Arg::with_name("case-sensitive")
                .long("case-sensitive")
                .short("s")
                .overrides_with_all(&["ignore-case", "case-sensitive"])
                .help("Case-sensitive search (default: smart case)")
                .long_help(
                    "Perform a case-sensitive search. By default, fd uses case-insensitive \
                         searches, unless the pattern contains an uppercase character (smart \
                         case).",
                ),
        )
        .arg(
            Arg::with_name("ignore-case")
                .long("ignore-case")
                .short("i")
                .overrides_with_all(&["case-sensitive", "ignore-case"])
                .help("Case-insensitive search (default: smart case)")
                .long_help(
                    "Perform a case-insensitive search. By default, fd uses case-insensitive \
                         searches, unless the pattern contains an uppercase character (smart \
                         case).",
                ),
        )
        .arg(
            Arg::with_name("glob")
                .long("glob")
                .short("g")
                .conflicts_with("fixed-strings")
                .overrides_with("glob")
                .help("Glob-based search (default: regular expression)")
                .long_help("Perform a glob-based search instead of a regular expression search."),
        )
        .arg(
            Arg::with_name("regex")
                .long("regex")
                .overrides_with_all(&["glob", "regex"])
                .hidden_short_help(true)
                .long_help(
                    "Perform a regular-expression based search (default). This can be used to \
                         override --glob.",
                ),
        )
        .arg(
            Arg::with_name("fixed-strings")
                .long("fixed-strings")
                .short("F")
                .alias("literal")
                .overrides_with("fixed-strings")
                .help("Treat the pattern as a literal string")
                .long_help(
                    "Treat the pattern as a literal string instead of a regular expression.",
                ),
        )
        .arg(
            Arg::with_name("absolute-path")
                .long("absolute-path")
                .short("a")
                .overrides_with("absolute-path")
                .help("Show absolute instead of relative paths")
                .long_help(
                    "Shows the full path starting from the root as opposed to relative paths.",
                ),
        )
        .arg(
            Arg::with_name("list-details")
                .long("list-details")
                .short("l")
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
            Arg::with_name("follow")
                .long("follow")
                .short("L")
                .alias("dereference")
                .overrides_with("follow")
                .help("Follow symbolic links")
                .long_help(
                    "By default, fd does not descend into symlinked directories. Using this \
                         flag, symbolic links are also traversed.",
                ),
        )
        .arg(
            Arg::with_name("full-path")
                .long("full-path")
                .short("p")
                .overrides_with("full-path")
                .help("Search full path (default: file-/dirname only)")
                .long_help(
                    "By default, the search pattern is only matched against the filename (or \
                         directory name). Using this flag, the pattern is matched against the \
                         full path.",
                ),
        )
        .arg(
            Arg::with_name("null_separator")
                .long("print0")
                .short("0")
                .overrides_with("print0")
                .conflicts_with("list-details")
                .help("Separate results by the null character")
                .long_help(
                    "Separate search results by the null character (instead of newlines). \
                         Useful for piping results to 'xargs'.",
                ),
        )
        .arg(
            Arg::with_name("depth")
                .long("max-depth")
                .short("d")
                .takes_value(true)
                .help("Set maximum search depth (default: none)")
                .long_help(
                    "Limit the directory traversal to a given depth. By default, there is no \
                         limit on the search depth.",
                ),
        )
        // support --maxdepth as well, for compatibility with rg
        .arg(
            Arg::with_name("rg-depth")
                .long("maxdepth")
                .hidden(true)
                .takes_value(true)
        )
        .arg(
            Arg::with_name("file-type")
                .long("type")
                .short("t")
                .multiple(true)
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
                ])
                .hide_possible_values(true)
                .help(
                    "Filter by type: file (f), directory (d), symlink (l),\nexecutable (x), \
                         empty (e)",
                )
                .long_help(
                    "Filter the search by type (multiple allowable filetypes can be specified):\n  \
                       'f' or 'file':         regular files\n  \
                       'd' or 'directory':    directories\n  \
                       'l' or 'symlink':      symbolic links\n  \
                       'x' or 'executable':   executables\n  \
                       'e' or 'empty':        empty files or directories",
                ),
        )
        .arg(
            Arg::with_name("extension")
                .long("extension")
                .short("e")
                .multiple(true)
                .number_of_values(1)
                .takes_value(true)
                .value_name("ext")
                .help("Filter by file extension")
                .long_help(
                    "(Additionally) filter search results by their file extension. Multiple \
                         allowable file extensions can be specified.",
                ),
        )
        .arg(
            Arg::with_name("exec")
                .long("exec")
                .short("x")
                .min_values(1)
                .allow_hyphen_values(true)
                .value_terminator(";")
                .value_name("cmd")
                .conflicts_with("list-details")
                .help("Execute a command for each search result")
                .long_help(
                    "Execute a command for each search result.\n\
                     All arguments following --exec are taken to be arguments to the command until the \
                     argument ';' is encountered.\n\
                     Each occurrence of the following placeholders is substituted by a path derived from the \
                     current search result before the command is executed:\n  \
                       '{}':   path\n  \
                       '{/}':  basename\n  \
                       '{//}': parent directory\n  \
                       '{.}':  path without file extension\n  \
                       '{/.}': basename without file extension",
                ),
        )
        .arg(
            Arg::with_name("exec-batch")
                .long("exec-batch")
                .short("X")
                .min_values(1)
                .allow_hyphen_values(true)
                .value_terminator(";")
                .value_name("cmd")
                .conflicts_with_all(&["exec", "list-details"])
                .help("Execute a command with all search results at once")
                .long_help(
                    "Execute a command with all search results at once.\n\
                     All arguments following --exec-batch are taken to be arguments to the command until the \
                     argument ';' is encountered.\n\
                     A single occurrence of the following placeholders is authorized and substituted by the paths derived from the \
                     search results before the command is executed:\n  \
                       '{}':   path\n  \
                       '{/}':  basename\n  \
                       '{//}': parent directory\n  \
                       '{.}':  path without file extension\n  \
                       '{/.}': basename without file extension",
                ),
        )
        .arg(
            Arg::with_name("exclude")
                .long("exclude")
                .short("E")
                .takes_value(true)
                .value_name("pattern")
                .number_of_values(1)
                .multiple(true)
                .help("Exclude entries that match the given glob pattern")
                .long_help(
                    "Exclude files/directories that match the given glob pattern. This \
                         overrides any other ignore logic. Multiple exclude patterns can be \
                         specified.",
                ),
        )
        .arg(
            Arg::with_name("ignore-file")
                .long("ignore-file")
                .takes_value(true)
                .value_name("path")
                .number_of_values(1)
                .multiple(true)
                .hidden_short_help(true)
                .long_help(
                    "Add a custom ignore-file in '.gitignore' format. These files have a low \
                         precedence.",
                ),
        )
        .arg(
            Arg::with_name("color")
                .long("color")
                .short("c")
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
            Arg::with_name("threads")
                .long("threads")
                .short("j")
                .takes_value(true)
                .value_name("num")
                .hidden_short_help(true)
                .long_help(
                    "Set number of threads to use for searching & executing (default: number \
                         of available CPU cores)",
                ),
        )
        .arg(
            Arg::with_name("size")
                .long("size")
                .short("S")
                .takes_value(true)
                .number_of_values(1)
                .allow_hyphen_values(true)
                .multiple(true)
                .help("Limit results based on the size of files.")
                .long_help(
                    "Limit results based on the size of files using the format <+-><NUM><UNIT>.\n   \
                        '+': file size must be greater than or equal to this\n   \
                        '-': file size must be less than or equal to this\n   \
                        'NUM':  The numeric size (e.g. 500)\n   \
                        'UNIT': The units for NUM. They are not case-sensitive.\n\
                     Allowed unit values:\n   \
                         'b':  bytes\n   \
                         'k':  kilobytes\n   \
                         'm':  megabytes\n   \
                         'g':  gigabytes\n   \
                         't':  terabytes\n   \
                         'ki': kibibytes\n   \
                         'mi': mebibytes\n   \
                         'gi': gibibytes\n   \
                         'ti': tebibytes",
                ),
        )
        .arg(
            Arg::with_name("perm")
                .long("perm")
                .short("P")
                .takes_value(true)
                .number_of_values(1)
                .allow_hyphen_values(false)
                .multiple(false)
                .help("Limit results based on the files permissions.")
                .long_help("Limit results based on the files permissions using the 3 digits numeric notation."),
        )
        .arg(
            Arg::with_name("max-buffer-time")
                .long("max-buffer-time")
                .takes_value(true)
                .hidden(true)
                .long_help(
                    "Amount of time in milliseconds to buffer, before streaming the search \
                         results to the console.",
                ),
        )
        .arg(
            Arg::with_name("changed-within")
                .long("changed-within")
                .alias("change-newer-than")
                .takes_value(true)
                .value_name("date|dur")
                .number_of_values(1)
                .help("Filter by file modification time (newer than)")
                .long_help(
                    "Filter results based on the file modification time. The argument can be provided \
                     as a specific point in time (YYYY-MM-DD HH:MM:SS) or as a duration (10h, 1d, 35min). \
                     '--change-newer-than' can be used as an alias.\n\
                     Examples:\n    \
                         --changed-within 2weeks\n    \
                         --change-newer-than '2018-10-27 10:00:00'",
                ),
        )
        .arg(
            Arg::with_name("changed-before")
                .long("changed-before")
                .alias("change-older-than")
                .takes_value(true)
                .value_name("date|dur")
                .number_of_values(1)
                .help("Filter by file modification time (older than)")
                .long_help(
                    "Filter results based on the file modification time. The argument can be provided \
                     as a specific point in time (YYYY-MM-DD HH:MM:SS) or as a duration (10h, 1d, 35min). \
                     '--change-older-than' can be used as an alias.\n\
                     Examples:\n    \
                         --changed-before '2018-10-27 10:00:00'\n    \
                         --change-older-than 2weeks",
                ),
        )
        .arg(
            Arg::with_name("max-results")
                .long("max-results")
                .takes_value(true)
                .value_name("count")
                .conflicts_with_all(&["exec", "exec-batch"])
                .hidden_short_help(true)
                .long_help("Limit the number of search results to 'count' and quit immediately."),
        )
        .arg(
            Arg::with_name("show-errors")
                .long("show-errors")
                .hidden_short_help(true)
                .overrides_with("show-errors")
                .long_help(
                    "Enable the display of filesystem errors for situations such as \
                         insufficient permissions or dead symlinks.",
                ),
        )
        .arg(
            Arg::with_name("base-directory")
                .long("base-directory")
                .takes_value(true)
                .value_name("path")
                .number_of_values(1)
                .hidden_short_help(true)
                .long_help(
                    "Change the current working directory of fd to the provided path. The \
                         means that search results will be shown with respect to the given base \
                         path. Note that relative paths which are passed to fd via the positional \
                         <path> argument or the '--search-path' option will also be resolved \
                         relative to this directory.",
                ),
        )
        .arg(
            Arg::with_name("pattern").help(
                "the search pattern - a regular expression unless '--glob' is used (optional)",
            ),
        )
        .arg(
            Arg::with_name("path-separator")
                .takes_value(true)
                .value_name("separator")
                .long("path-separator")
                .hidden_short_help(true)
                .long_help(
                    "Set the path separator to use when printing file paths. The default is \
                         the OS-specific separator ('/' on Unix, '\\' on Windows).",
                ),
        )
        .arg(
            Arg::with_name("path")
                .multiple(true)
                .help("the root directory for the filesystem search (optional)")
                .long_help(
                    "The directory where the filesystem search is rooted (optional). If \
                         omitted, search the current working directory.",
                ),
        )
        .arg(
            Arg::with_name("search-path")
                .long("search-path")
                .takes_value(true)
                .conflicts_with("path")
                .multiple(true)
                .hidden_short_help(true)
                .number_of_values(1)
                .long_help(
                    "Provide paths to search as an alternative to the positional <path> \
                         argument. Changes the usage to `fd [FLAGS/OPTIONS] --search-path <path> \
                         --search-path <path2> [<pattern>]`",
                ),
        );

    // Make `--one-file-system` available only on Unix and Windows platforms, as per the
    // restrictions on the corresponding option in the `ignore` crate.
    // Provide aliases `mount` and `xdev` for people coming from `find`.
    if cfg!(any(unix, windows)) {
        app = app.arg(
            Arg::with_name("one-file-system")
                .long("one-file-system")
                .aliases(&["mount", "xdev"])
                .hidden_short_help(true)
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
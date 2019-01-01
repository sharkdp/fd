// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.
use std::collections::HashMap;

use clap::{App, AppSettings, Arg};

struct Help {
    short: &'static str,
    long: &'static str,
}

macro_rules! doc {
    ($map:expr, $name:expr, $short:expr) => {
        doc!($map, $name, $short, $short)
    };
    ($map:expr, $name:expr, $short:expr, $long:expr) => {
        $map.insert(
            $name,
            Help {
                short: $short,
                long: concat!($long, "\n "),
            },
        );
    };
}

pub fn build_app() -> App<'static, 'static> {
    let helps = usage();
    let arg = |name| {
        Arg::with_name(name)
            .help(helps[name].short)
            .long_help(helps[name].long)
    };

    App::new("fd")
        .version(crate_version!())
        .usage("fd [FLAGS/OPTIONS] [<pattern>] [<path>...]")
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .after_help("Note: `fd -h` prints a short and concise overview while `fd --help` \
                     gives all details.")
        .arg(arg("hidden").long("hidden").short("H"))
        .arg(arg("no-ignore").long("no-ignore").short("I"))
        .arg(arg("no-ignore-vcs").long("no-ignore-vcs"))
        .arg(
            arg("rg-alias-hidden-ignore")
                .short("u")
                .multiple(true)
                .hidden(true),
        )
        .arg(
            arg("case-sensitive")
                .long("case-sensitive")
                .short("s")
                .overrides_with("ignore-case"),
        )
        .arg(
            arg("ignore-case")
                .long("ignore-case")
                .short("i")
                .overrides_with("case-sensitive"),
        )
        .arg(
            arg("fixed-strings")
                .long("fixed-strings")
                .short("F")
                .alias("literal"),
        )
        .arg(arg("absolute-path").long("absolute-path").short("a"))
        .arg(arg("follow").long("follow").short("L").alias("dereference"))
        .arg(arg("full-path").long("full-path").short("p"))
        .arg(arg("null_separator").long("print0").short("0"))
        .arg(arg("depth").long("max-depth").short("d").takes_value(true))
        // support --maxdepth as well, for compatibility with rg
        .arg(
            arg("rg-depth")
                .long("maxdepth")
                .hidden(true)
                .takes_value(true),
        )
        .arg(
            arg("file-type")
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
                .hide_possible_values(true),
        )
        .arg(
            arg("extension")
                .long("extension")
                .short("e")
                .multiple(true)
                .number_of_values(1)
                .takes_value(true)
                .value_name("ext"),
        )
        .arg(
            arg("exec")
                .long("exec")
                .short("x")
                .min_values(1)
                .allow_hyphen_values(true)
                .value_terminator(";")
                .value_name("cmd"),
        )
        .arg(
            arg("exec-batch")
                .long("exec-batch")
                .short("X")
                .min_values(1)
                .allow_hyphen_values(true)
                .value_terminator(";")
                .value_name("cmd")
                .conflicts_with("exec"),
        )
        .arg(
            arg("exclude")
                .long("exclude")
                .short("E")
                .takes_value(true)
                .value_name("pattern")
                .number_of_values(1)
                .multiple(true),
        )
        .arg(
            arg("ignore-file")
                .long("ignore-file")
                .takes_value(true)
                .value_name("path")
                .number_of_values(1)
                .multiple(true)
                .hidden_short_help(true),
        )
        .arg(
            arg("color")
                .long("color")
                .short("c")
                .takes_value(true)
                .value_name("when")
                .possible_values(&["never", "auto", "always"])
                .hide_possible_values(true),
        )
        .arg(
            arg("threads")
                .long("threads")
                .short("j")
                .takes_value(true)
                .value_name("num")
                .hidden_short_help(true),
        )
        .arg(
            arg("size")
                .long("size")
                .short("S")
                .takes_value(true)
                .number_of_values(1)
                .allow_hyphen_values(true)
                .multiple(true),
        )
        .arg(
            arg("max-buffer-time")
                .long("max-buffer-time")
                .takes_value(true)
                .hidden(true),
        )
        .arg(
            arg("changed-within")
                .long("changed-within")
                .alias("change-newer-than")
                .takes_value(true)
                .value_name("date|dur")
                .number_of_values(1),
        )
        .arg(
            arg("changed-before")
                .long("changed-before")
                .alias("change-older-than")
                .takes_value(true)
                .value_name("date|dur")
                .number_of_values(1),
        )
        .arg(
            arg("show-errors")
                .long("show-errors")
                .hidden_short_help(true),
        )
        .arg(arg("pattern"))
        .arg(arg("path").multiple(true))
        .arg(
            arg("search-path")
                .long("search-path")
                .takes_value(true)
                .conflicts_with("path")
                .multiple(true)
                .hidden_short_help(true)
                .number_of_values(1),
        )
}

#[cfg_attr(rustfmt, rustfmt_skip)]
fn usage() -> HashMap<&'static str, Help> {
    let mut h = HashMap::new();
    doc!(h, "hidden"
        , "Search hidden files and directories"
        , "Include hidden directories and files in the search results (default: hidden files \
           and directories are skipped). Files and directories are considered to be hidden if \
           their name starts with a `.` sign (dot).");
    doc!(h, "no-ignore"
        , "Do not respect .(git|fd)ignore files"
        , "Show search results from files and directories that would otherwise be ignored by \
            '.gitignore', '.ignore' or '.fdignore' files.");
    doc!(h, "no-ignore-vcs"
        , "Do not respect .gitignore files"
        , "Show search results from files and directories that would otherwise be ignored by \
            '.gitignore' files.");
    doc!(h, "case-sensitive"
        , "Case-sensitive search (default: smart case)"
        , "Perform a case-sensitive search. By default, fd uses case-insensitive searches, \
           unless the pattern contains an uppercase character (smart case).");
    doc!(h, "ignore-case"
        , "Case-insensitive search (default: smart case)"
        , "Perform a case-insensitive search. By default, fd uses case-insensitive searches, \
           unless the pattern contains an uppercase character (smart case).");
    doc!(h, "fixed-strings"
        , "Treat the pattern as a literal string"
        , "Treat the pattern as a literal string instead of a regular expression.");
    doc!(h, "absolute-path"
        , "Show absolute instead of relative paths"
        , "Shows the full path starting from the root as opposed to relative paths.");
    doc!(h, "follow"
        , "Follow symbolic links"
        , "By default, fd does not descend into symlinked directories. Using this flag, symbolic \
           links are also traversed.");
    doc!(h, "full-path"
        , "Search full path (default: file-/dirname only)"
        , "By default, the search pattern is only matched against the filename (or directory \
           name). Using this flag, the pattern is matched against the full path.");
    doc!(h, "null_separator"
        , "Separate results by the null character"
        , "Separate search results by the null character (instead of newlines). Useful for \
           piping results to 'xargs'.");
    doc!(h, "depth"
        , "Set maximum search depth (default: none)"
        , "Limit the directory traversal to a given depth. By default, there is no limit \
           on the search depth.");
    doc!(h, "rg-depth"
        , "See --max-depth"
        , "See --max-depth");
    doc!(h, "file-type"
        , "Filter by type: file (f), directory (d), symlink (l),\nexecutable (x), empty (e)"
        , "Filter the search by type (multiple allowable filetypes can be specified):\n  \
             'f' or 'file':         regular files\n  \
             'd' or 'directory':    directories\n  \
             'l' or 'symlink':      symbolic links\n  \
             'x' or 'executable':   executables\n  \
             'e' or 'empty':        empty files or directories");
    doc!(h, "extension"
        , "Filter by file extension"
        , "(Additionally) filter search results by their file extension. Multiple allowable file \
           extensions can be specified.");
    doc!(h, "exec"
        , "Execute a command for each search result"
        , "Execute a command for each search result.\n\
           All arguments following --exec are taken to be arguments to the command until the \
           argument ';' is encountered.\n\
           Each occurrence of the following placeholders is substituted by a path derived from the \
           current search result before the command is executed:\n  \
             '{}':   path\n  \
             '{/}':  basename\n  \
             '{//}': parent directory\n  \
             '{.}':  path without file extension\n  \
             '{/.}': basename without file extension");
    doc!(h, "exec-batch"
        , "Execute a command with all search results at once"
        , "Execute a command with all search results at once.\n\
           All arguments following --exec-batch are taken to be arguments to the command until the \
           argument ';' is encountered.\n\
           A single occurence of the following placeholders is authorized and substituted by the paths derived from the \
           search results before the command is executed:\n  \
             '{}':   path\n  \
             '{/}':  basename\n  \
             '{//}': parent directory\n  \
             '{.}':  path without file extension\n  \
             '{/.}': basename without file extension");
    doc!(h, "exclude"
        , "Exclude entries that match the given glob pattern"
        , "Exclude files/directories that match the given glob pattern. This overrides any \
           other ignore logic. Multiple exclude patterns can be specified.");
    doc!(h, "ignore-file"
        , "Add a custom ignore-file in .gitignore format"
        , "Add a custom ignore-file in '.gitignore' format. These files have a low precedence.");
    doc!(h, "color"
        , "When to use colors: never, *auto*, always"
        , "Declare when to use color for the pattern match output:\n  \
             'auto':      show colors if the output goes to an interactive console (default)\n  \
             'never':     do not use colorized output\n  \
             'always':    always use colorized output");
    doc!(h, "threads"
        , "Set number of threads to use for searching & executing"
        , "Set number of threads to use for searching & executing (default: number of available \
           CPU cores)");
    doc!(h, "max-buffer-time"
        , "the time (in ms) to buffer, before streaming to the console"
        , "Amount of time in milliseconds to buffer, before streaming the search results to \
           the console.");
    doc!(h, "pattern"
        , "the search pattern, a regular expression (optional)");
    doc!(h, "path"
        , "the root directory for the filesystem search (optional)"
        , "The directory where the filesystem search is rooted (optional). \
           If omitted, search the current working directory.");
    doc!(h, "rg-alias-hidden-ignore"
        , "Alias for no-ignore and/or hidden"
        , "Alias for no-ignore ('u') and no-ignore and hidden ('uu')");
    doc!(h, "size"
        , "Limit results based on the size of files."
        , "Limit results based on the size of files using the format <+-><NUM><UNIT>.\n   \
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
                'ti': tebibytes");
    doc!(h, "changed-within"
        , "Filter by file modification time (newer than)"
        , "Filter results based on the file modification time. The argument can be provided \
           as a specific point in time (YYYY-MM-DD HH:MM:SS) or as a duration (10h, 1d, 35min). \
           '--change-newer-than' can be used as an alias.\n\
           Examples:\n    \
               --changed-within 2weeks\n    \
               --change-newer-than '2018-10-27 10:00:00'");
    doc!(h, "changed-before"
        , "Filter by file modification time (older than)"
        , "Filter results based on the file modification time. The argument can be provided \
           as a specific point in time (YYYY-MM-DD HH:MM:SS) or as a duration (10h, 1d, 35min). \
           '--change-older-than' can be used as an alias.\n\
           Examples:\n    \
               --changed-before '2018-10-27 10:00:00'\n    \
               --change-older-than 2weeks");
    doc!(h, "show-errors"
        , "Enable display of filesystem errors"
        , "Enable the display of filesystem errors for situations such as insufficient permissions \
            or dead symlinks.");
    doc!(h, "search-path"
        , "(hidden)"
        , "Provide paths to search as an alternative to the positional <path> argument. \
           Changes the usage to `fd [FLAGS/OPTIONS] --search-path <path> --search-path <path2> [<pattern>]`");
    h
}

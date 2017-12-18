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
        $map.insert($name, Help {
            short: $short,
            long: concat!($long, "\n ")
        });
    };
}

pub fn build_app() -> App<'static, 'static> {
    let helps = usage();
    let arg = |name| {
        Arg::with_name(name).help(helps[name].short).long_help(
            helps[name].long,
        )
    };

    App::new("fd")
        .version(crate_version!())
        .usage("fd [FLAGS/OPTIONS] [<pattern>] [<path>...]")
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
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
        .arg(arg("absolute-path").long("absolute-path").short("a"))
        .arg(arg("follow").long("follow").short("L").alias("dereference"))
        .arg(arg("full-path").long("full-path").short("p"))
        .arg(arg("null_separator").long("print0").short("0"))
        .arg(arg("depth").long("max-depth").short("d").takes_value(true))
        .arg(
            arg("file-type")
                .long("type")
                .short("t")
                .takes_value(true)
                .value_name("filetype")
                .possible_values(&["f", "file", "d", "directory", "l", "symlink"])
                .hide_possible_values(true),
        )
        .arg(
            arg("extension")
                .long("extension")
                .short("e")
                .takes_value(true)
                .value_name("ext"),
        )
        .arg(
            arg("exec")
                .long("exec")
                .short("x")
                .multiple(true)
                .min_values(1)
                .allow_hyphen_values(true)
                .value_terminator(";")
                .value_name("cmd"),
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
                .value_name("num"),
        )
        .arg(
            arg("max-buffer-time")
                .long("max-buffer-time")
                .takes_value(true)
                .hidden(true),
        )
        .arg(arg("pattern"))
        .arg(arg("path").multiple(true))
}

#[cfg_attr(rustfmt, rustfmt_skip)]
fn usage() -> HashMap<&'static str, Help> {
    let mut h = HashMap::new();
    doc!(h, "hidden"
        , "Search hidden files and directories"
        , "Include hidden directories and files in the search results (default: hidden files \
           and directories are skipped).");
    doc!(h, "no-ignore"
        , "Do not respect .(git)ignore files"
        , "Show search results from files and directories that would otherwise be ignored by \
            '.*ignore' files.");
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
    doc!(h, "file-type"
        , "Filter by type: f(ile), d(irectory), (sym)l(ink)"
        , "Filter the search by type:\n  \
             'f' or 'file':         regular files\n  \
             'd' or 'directory':    directories\n  \
             'l' or 'symlink':      symbolic links");
    doc!(h, "extension"
        , "Filter by file extension"
        , "(Additionally) filter search results by their file extension.");
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
    doc!(h, "exclude"
        , "Exclude entries that match the given glob pattern."
        , "Exclude files/directories that match the given glob pattern. This overrides any \
           other ignore logic. Multiple exclude patterns can be specified.");
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
        , "Amount of time in milliseconds to buffer, before streaming the search results to\
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

    h
}

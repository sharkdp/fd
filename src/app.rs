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
        .usage("fd [FLAGS/OPTIONS] [<pattern>] [<path>]")
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(arg("hidden").long("hidden").short("H"))
        .arg(arg("no-ignore").long("no-ignore").short("I"))
        .arg(arg("case-sensitive").long("case-sensitive").short("s"))
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
                .possible_values(&["f", "file", "d", "directory", "s", "symlink"])
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
        .arg(arg("path"))
}

#[cfg_attr(rustfmt, rustfmt_skip)]
fn usage() -> HashMap<&'static str, Help> {
    let mut h = HashMap::new();
    doc!(h, "hidden"
        , "Search hidden files and directories"
        , "Also include hidden directories and files in the search results (default: hidden files\
            and directories are skipped).");
    doc!(h, "no-ignore"
        , "Do not respect .(git)ignore files"
        , "Show search results from files and directories that would otherwise be ignored by\
            '.*ignore' files.");
    doc!(h, "case-sensitive"
        , "Case-sensitive search (default: smart case)"
        , "Define your pattern as case sensitive. By default, fd uses 'smart case' for queries.\
            This option disables smart case.");
    doc!(h, "absolute-path"
        , "Show absolute instead of relative paths"
        , "Shows the result of a pattern match as an absolute path instead of relative path.");
    doc!(h, "follow"
        , "Follow symbolic links"
        , "By default, fd does not descent into symlinked directories. Using this flag, symbolic \
            links are also traversed.");
    doc!(h, "full-path"
        , "Search full path (default: file-/dirname only)"
        , "Searches the full path of directory. By default, fd only searches the last part of \
            the path (the file name or the directory name).");
    doc!(h, "null_separator"
        , "Separate results by the null character"
        , "Separate search results by the null character (instead of newlines). This is useful \
            for piping results to 'xargs'.");
    doc!(h, "depth"
        , "Set maximum search depth (default: none)"
        , "Set the limit of the maximum search depth in a pattern match query. By default, there\
            is no limit on the search depth.");
    doc!(h, "file-type"
        , "Filter by type: f(ile), d(irectory), s(ymlink)"
        , "Filter the search by type:\n\
            f file          for file\n\
            d directory     for directory\n\
            s symlink       for symbolic links");
    doc!(h, "extension"
        , "Filter by file extension"
        , "Only show search results with a specific file extension.");
    doc!(h, "color"
        , "When to use color in the output:\n\
            never, auto, always (default: auto)"
        , "Declare when to use color for the pattern match output:\n\
            auto (default)\n\
            never\n\
            always");
    doc!(h, "threads"
        , "Set number of threads to use for searching:\n\
            (default: number of available CPU cores)");
    doc!(h, "max-buffer-time"
        , "the time (in ms) to buffer, before streaming to the console"
        , "Amount of time in milliseconds to buffer, before streaming the search results to\
            the console.");
    doc!(h, "pattern"
        , "the search pattern, a regular expression (optional)");
    doc!(h, "path"
        , "the root directory for the filesystem search (optional)"
        , "The root directory where you want to do the filesystem search of the pattern.\
            When not present, the default is to use the current working directory.");

    h
}

use clap::{App, AppSettings, Arg};

pub fn build_app() -> App<'static, 'static> {
    App::new("fd")
        .version(crate_version!())
        .usage("fd [FLAGS/OPTIONS] [<pattern>] [<path>]")
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::with_name("hidden")
                .long("hidden")
                .short("H")
                .help("Search hidden files and directories"),
        )
        .arg(
            Arg::with_name("no-ignore")
                .long("no-ignore")
                .short("I")
                .help("Do not respect .(git)ignore files"),
        )
        .arg(
            Arg::with_name("case-sensitive")
                .long("case-sensitive")
                .short("s")
                .help("Case-sensitive search (default: smart case)"),
        )
        .arg(
            Arg::with_name("absolute-path")
                .long("absolute-path")
                .short("a")
                .help("Show absolute instead of relative paths"),
        )
        .arg(
            Arg::with_name("follow")
                .long("follow")
                .short("L")
                .alias("dereference")
                .help("Follow symbolic links"),
        )
        .arg(
            Arg::with_name("full-path")
                .long("full-path")
                .short("p")
                .help("Search full path (default: file-/dirname only)"),
        )
        .arg(
            Arg::with_name("null_separator")
                .long("print0")
                .short("0")
                .help("Separate results by the null character"),
        )
        .arg(
            Arg::with_name("depth")
                .long("max-depth")
                .short("d")
                .takes_value(true)
                .help("Set maximum search depth (default: none)"),
        )
        .arg(
            Arg::with_name("file-type")
                .long("type")
                .short("t")
                .takes_value(true)
                .possible_values(&["f", "file", "d", "directory", "s", "symlink"])
                .hide_possible_values(true)
                .help("Filter by type: f(ile), d(irectory), s(ymlink)"),
        )
        .arg(
            Arg::with_name("extension")
                .long("extension")
                .short("e")
                .takes_value(true)
                .value_name("ext")
                .help("Filter by file extension"),
        )
        .arg(
            Arg::with_name("color")
                .long("color")
                .short("c")
                .takes_value(true)
                .possible_values(&["never", "auto", "always"])
                .hide_possible_values(true)
                .help(
                    "When to use color in the output:\n\
                     never, auto, always (default: auto)",
                ),
        )
        .arg(
            Arg::with_name("threads")
                .long("threads")
                .short("j")
                .takes_value(true)
                .help(
                    "Set number of threads to use for searching\n\
                     (default: number of available CPU cores)",
                ),
        )
        .arg(
            Arg::with_name("max-buffer-time")
                .long("max-buffer-time")
                .takes_value(true)
                .hidden(true)
                .help("the time (in ms) to buffer, before streaming to the console"),
        )
        .arg(Arg::with_name("pattern").help("the search pattern, a regular expression (optional)"))
        .arg(Arg::with_name("path").help("the root directory for the filesystem search (optional)"))
}

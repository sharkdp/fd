mod cli;
mod config;
mod dir_entry;
mod error;
mod exec;
mod exit_codes;
mod filesystem;
mod filetypes;
mod filter;
mod fmt;
mod hyperlink;
mod output;
mod regex_helper;
mod walk;

use std::env;
use std::io::IsTerminal;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use clap::{CommandFactory, Parser};
use globset::GlobBuilder;
use lscolors::LsColors;
use regex::bytes::{Regex, RegexBuilder, RegexSetBuilder};

use crate::cli::{ColorWhen, HyperlinkWhen, Opts};
use crate::config::Config;
use crate::exec::CommandSet;
use crate::exit_codes::ExitCode;
use crate::filetypes::FileTypes;
#[cfg(unix)]
use crate::filter::OwnerFilter;
use crate::filter::TimeFilter;
use crate::regex_helper::{pattern_has_uppercase_char, pattern_matches_strings_with_leading_dot};

// We use jemalloc for performance reasons, see https://github.com/sharkdp/fd/pull/481
// FIXME: re-enable jemalloc on macOS, see comment in Cargo.toml file for more infos
// This has to be kept in sync with the Cargo.toml file section that declares a
// dependency on tikv-jemallocator.
#[cfg(all(
    not(windows),
    not(target_os = "android"),
    not(target_os = "macos"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd"),
    not(target_os = "illumos"),
    not(all(target_env = "musl", target_pointer_width = "32")),
    not(target_arch = "riscv64"),
    feature = "use-jemalloc"
))]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// dircolors | grep -oP "LS_COLORS='\K[^']+"
const DEFAULT_LS_COLORS: &str = "
rs=0:di=01;34:ln=01;36:mh=00:pi=40;33:so=01;35:do=01;35:bd=40;33;01:cd=40;33;01:or=40;31;01:mi=00:su=37;41:sg=30;43:ca=00:tw=30;42:ow=34;42:st=37;44:ex=01;32:*.7z=01;31:*.ace=01;31:*.alz=01;31:*.apk=01;31:*.arc=01;31:*.arj=01;31:*.bz=01;31:*.bz2=01;31:*.cab=01;31:*.cpio=01;31:*.crate=01;31:*.deb=01;31:*.drpm=01;31:*.dwm=01;31:*.dz=01;31:*.ear=01;31:*.egg=01;31:*.esd=01;31:*.gz=01;31:*.jar=01;31:*.lha=01;31:*.lrz=01;31:*.lz=01;31:*.lz4=01;31:*.lzh=01;31:*.lzma=01;31:*.lzo=01;31:*.pyz=01;31:*.rar=01;31:*.rpm=01;31:*.rz=01;31:*.sar=01;31:*.swm=01;31:*.t7z=01;31:*.tar=01;31:*.taz=01;31:*.tbz=01;31:*.tbz2=01;31:*.tgz=01;31:*.tlz=01;31:*.txz=01;31:*.tz=01;31:*.tzo=01;31:*.tzst=01;31:*.udeb=01;31:*.war=01;31:*.whl=01;31:*.wim=01;31:*.xz=01;31:*.z=01;31:*.zip=01;31:*.zoo=01;31:*.zst=01;31:*.avif=01;35:*.jpg=01;35:*.jpeg=01;35:*.jxl=01;35:*.mjpg=01;35:*.mjpeg=01;35:*.gif=01;35:*.bmp=01;35:*.pbm=01;35:*.pgm=01;35:*.ppm=01;35:*.tga=01;35:*.xbm=01;35:*.xpm=01;35:*.tif=01;35:*.tiff=01;35:*.png=01;35:*.svg=01;35:*.svgz=01;35:*.mng=01;35:*.pcx=01;35:*.mov=01;35:*.mpg=01;35:*.mpeg=01;35:*.m2v=01;35:*.mkv=01;35:*.webm=01;35:*.webp=01;35:*.ogm=01;35:*.mp4=01;35:*.m4v=01;35:*.mp4v=01;35:*.vob=01;35:*.qt=01;35:*.nuv=01;35:*.wmv=01;35:*.asf=01;35:*.rm=01;35:*.rmvb=01;35:*.flc=01;35:*.avi=01;35:*.fli=01;35:*.flv=01;35:*.gl=01;35:*.dl=01;35:*.xcf=01;35:*.xwd=01;35:*.yuv=01;35:*.cgm=01;35:*.emf=01;35:*.ogv=01;35:*.ogx=01;35:*.aac=00;36:*.au=00;36:*.flac=00;36:*.m4a=00;36:*.mid=00;36:*.midi=00;36:*.mka=00;36:*.mp3=00;36:*.mpc=00;36:*.ogg=00;36:*.ra=00;36:*.wav=00;36:*.oga=00;36:*.opus=00;36:*.spx=00;36:*.xspf=00;36:*~=00;90:*#=00;90:*.bak=00;90:*.crdownload=00;90:*.dpkg-dist=00;90:*.dpkg-new=00;90:*.dpkg-old=00;90:*.dpkg-tmp=00;90:*.old=00;90:*.orig=00;90:*.part=00;90:*.rej=00;90:*.rpmnew=00;90:*.rpmorig=00;90:*.rpmsave=00;90:*.swp=00;90:*.tmp=00;90:*.ucf-dist=00;90:*.ucf-new=00;90:*.ucf-old=00;90:;
";

fn main() {
    let result = run();
    match result {
        Ok(exit_code) => {
            exit_code.exit();
        }
        Err(err) => {
            eprintln!("[fd error]: {err:#}");
            ExitCode::GeneralError.exit();
        }
    }
}

fn run() -> Result<ExitCode> {
    let opts = Opts::parse();

    #[cfg(feature = "completions")]
    if let Some(shell) = opts.gen_completions()? {
        return print_completions(shell);
    }

    set_working_dir(&opts)?;
    let search_paths = opts.search_paths()?;
    if search_paths.is_empty() {
        bail!("No valid search paths given.");
    }

    ensure_search_pattern_is_not_a_path(&opts)?;
    let pattern = &opts.pattern;
    let exprs = &opts.exprs;
    let empty = Vec::new();

    let pattern_regexps = exprs
        .as_ref()
        .unwrap_or(&empty)
        .iter()
        .chain([pattern])
        .map(|pat| build_pattern_regex(pat, &opts))
        .collect::<Result<Vec<String>>>()?;

    let config = construct_config(opts, &pattern_regexps)?;

    ensure_use_hidden_option_for_leading_dot_pattern(&config, &pattern_regexps)?;

    let regexps = pattern_regexps
        .into_iter()
        .map(|pat| build_regex(pat, &config))
        .collect::<Result<Vec<Regex>>>()?;

    walk::scan(&search_paths, regexps, config)
}

#[cfg(feature = "completions")]
#[cold]
fn print_completions(shell: clap_complete::Shell) -> Result<ExitCode> {
    // The program name is the first argument.
    let first_arg = env::args().next();
    let program_name = first_arg
        .as_ref()
        .map(Path::new)
        .and_then(|path| path.file_stem())
        .and_then(|file| file.to_str())
        .unwrap_or("fd");
    let mut cmd = Opts::command();
    cmd.build();
    clap_complete::generate(shell, &mut cmd, program_name, &mut std::io::stdout());
    Ok(ExitCode::Success)
}

fn set_working_dir(opts: &Opts) -> Result<()> {
    if let Some(ref base_directory) = opts.base_directory {
        if !filesystem::is_existing_directory(base_directory) {
            return Err(anyhow!(
                "The '--base-directory' path '{}' is not a directory.",
                base_directory.to_string_lossy()
            ));
        }
        env::set_current_dir(base_directory).with_context(|| {
            format!(
                "Could not set '{}' as the current working directory",
                base_directory.to_string_lossy()
            )
        })?;
    }
    Ok(())
}

/// Detect if the user accidentally supplied a path instead of a search pattern
fn ensure_search_pattern_is_not_a_path(opts: &Opts) -> Result<()> {
    if !opts.full_path
        && opts.pattern.contains(std::path::MAIN_SEPARATOR)
        && Path::new(&opts.pattern).is_dir()
    {
        Err(anyhow!(
            "The search pattern '{pattern}' contains a path-separation character ('{sep}') \
             and will not lead to any search results.\n\n\
             If you want to search for all files inside the '{pattern}' directory, use a match-all pattern:\n\n  \
             fd . '{pattern}'\n\n\
             Instead, if you want your pattern to match the full file path, use:\n\n  \
             fd --full-path '{pattern}'",
            pattern = &opts.pattern,
            sep = std::path::MAIN_SEPARATOR,
        ))
    } else {
        Ok(())
    }
}

fn build_pattern_regex(pattern: &str, opts: &Opts) -> Result<String> {
    Ok(if opts.glob && !pattern.is_empty() {
        let glob = GlobBuilder::new(pattern).literal_separator(true).build()?;
        glob.regex().to_owned()
    } else if opts.fixed_strings {
        // Treat pattern as literal string if '--fixed-strings' is used
        regex::escape(pattern)
    } else {
        String::from(pattern)
    })
}

fn check_path_separator_length(path_separator: Option<&str>) -> Result<()> {
    match (cfg!(windows), path_separator) {
        (true, Some(sep)) if sep.len() > 1 => Err(anyhow!(
            "A path separator must be exactly one byte, but \
                 the given separator is {} bytes: '{}'.\n\
                 In some shells on Windows, '/' is automatically \
                 expanded. Try to use '//' instead.",
            sep.len(),
            sep
        )),
        _ => Ok(()),
    }
}

fn construct_config(mut opts: Opts, pattern_regexps: &[String]) -> Result<Config> {
    // The search will be case-sensitive if the command line flag is set or
    // if any of the patterns has an uppercase character (smart case).
    let case_sensitive = !opts.ignore_case
        && (opts.case_sensitive
            || pattern_regexps
                .iter()
                .any(|pat| pattern_has_uppercase_char(pat)));

    let path_separator = opts
        .path_separator
        .take()
        .or_else(filesystem::default_path_separator);
    let actual_path_separator = path_separator
        .clone()
        .unwrap_or_else(|| std::path::MAIN_SEPARATOR.to_string());
    check_path_separator_length(path_separator.as_deref())?;

    let size_limits = std::mem::take(&mut opts.size);
    let time_constraints = extract_time_constraints(&opts)?;
    #[cfg(unix)]
    let owner_constraint: Option<OwnerFilter> = opts.owner.and_then(OwnerFilter::filter_ignore);

    #[cfg(windows)]
    let ansi_colors_support =
        nu_ansi_term::enable_ansi_support().is_ok() || std::env::var_os("TERM").is_some();
    #[cfg(not(windows))]
    let ansi_colors_support = true;

    let interactive_terminal = std::io::stdout().is_terminal();

    let colored_output = match opts.color {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => {
            let no_color = env::var_os("NO_COLOR").is_some_and(|x| !x.is_empty());
            ansi_colors_support && !no_color && interactive_terminal
        }
    };

    let ls_colors = if colored_output {
        Some(LsColors::from_env().unwrap_or_else(|| LsColors::from_string(DEFAULT_LS_COLORS)))
    } else {
        None
    };
    let hyperlink = match opts.hyperlink {
        HyperlinkWhen::Always => true,
        HyperlinkWhen::Never => false,
        HyperlinkWhen::Auto => colored_output,
    };
    let command = extract_command(&mut opts, colored_output)?;
    let has_command = command.is_some();

    Ok(Config {
        case_sensitive,
        search_full_path: opts.full_path,
        ignore_hidden: !(opts.hidden || opts.rg_alias_ignore()),
        read_fdignore: !(opts.no_ignore || opts.rg_alias_ignore()),
        read_vcsignore: !(opts.no_ignore || opts.rg_alias_ignore() || opts.no_ignore_vcs),
        require_git_to_read_vcsignore: !opts.no_require_git,
        read_parent_ignore: !opts.no_ignore_parent,
        read_global_ignore: !(opts.no_ignore
            || opts.rg_alias_ignore()
            || opts.no_global_ignore_file),
        follow_links: opts.follow,
        one_file_system: opts.one_file_system,
        null_separator: opts.null_separator,
        quiet: opts.quiet,
        max_depth: opts.max_depth(),
        min_depth: opts.min_depth(),
        prune: opts.prune,
        threads: opts.threads().get(),
        max_buffer_time: opts.max_buffer_time,
        ls_colors,
        hyperlink,
        interactive_terminal,
        file_types: opts.filetype.as_ref().map(|values| {
            use crate::cli::FileType::*;
            let mut file_types = FileTypes::default();
            for value in values {
                match value {
                    File => file_types.files = true,
                    Directory => file_types.directories = true,
                    Symlink => file_types.symlinks = true,
                    Executable => {
                        file_types.executables_only = true;
                        file_types.files = true;
                    }
                    Empty => file_types.empty_only = true,
                    BlockDevice => file_types.block_devices = true,
                    CharDevice => file_types.char_devices = true,
                    Socket => file_types.sockets = true,
                    Pipe => file_types.pipes = true,
                }
            }

            // If only 'empty' was specified, search for both files and directories:
            if file_types.empty_only && !(file_types.files || file_types.directories) {
                file_types.files = true;
                file_types.directories = true;
            }

            file_types
        }),
        extensions: opts
            .extensions
            .as_ref()
            .map(|exts| {
                let patterns = exts
                    .iter()
                    .map(|e| e.trim_start_matches('.'))
                    .map(|e| format!(r".\.{}$", regex::escape(e)));
                RegexSetBuilder::new(patterns)
                    .case_insensitive(true)
                    .build()
            })
            .transpose()?,
        format: opts
            .format
            .as_deref()
            .map(crate::fmt::FormatTemplate::parse),
        command: command.map(Arc::new),
        batch_size: opts.batch_size,
        exclude_patterns: opts.exclude.iter().map(|p| String::from("!") + p).collect(),
        ignore_files: std::mem::take(&mut opts.ignore_file),
        size_constraints: size_limits,
        time_constraints,
        #[cfg(unix)]
        owner_constraint,
        show_filesystem_errors: opts.show_errors,
        path_separator,
        actual_path_separator,
        max_results: opts.max_results(),
        strip_cwd_prefix: opts.strip_cwd_prefix(|| !(opts.null_separator || has_command)),
    })
}

fn extract_command(opts: &mut Opts, colored_output: bool) -> Result<Option<CommandSet>> {
    opts.exec
        .command
        .take()
        .map(Ok)
        .or_else(|| {
            if !opts.list_details {
                return None;
            }

            let res = determine_ls_command(colored_output)
                .map(|cmd| CommandSet::new_batch([cmd]).unwrap());
            Some(res)
        })
        .transpose()
}

fn determine_ls_command(colored_output: bool) -> Result<Vec<&'static str>> {
    #[allow(unused)]
    let gnu_ls = |command_name| {
        let color_arg = if colored_output {
            "--color=always"
        } else {
            "--color=never"
        };
        // Note: we use short options here (instead of --long-options) to support more
        // platforms (like BusyBox).
        vec![
            command_name,
            "-l", // long listing format
            "-h", // human readable file sizes
            "-d", // list directories themselves, not their contents
            color_arg,
        ]
    };
    let cmd: Vec<&str> = if cfg!(unix) {
        if !cfg!(any(
            target_os = "macos",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )) {
            // Assume ls is GNU ls
            gnu_ls("ls")
        } else {
            // MacOS, DragonFlyBSD, FreeBSD
            use std::process::{Command, Stdio};

            // Use GNU ls, if available (support for --color=auto, better LS_COLORS support)
            let gnu_ls_exists = Command::new("gls")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok();

            if gnu_ls_exists {
                gnu_ls("gls")
            } else {
                let mut cmd = vec![
                    "ls", // BSD version of ls
                    "-l", // long listing format
                    "-h", // '--human-readable' is not available, '-h' is
                    "-d", // '--directory' is not available, but '-d' is
                ];

                if !cfg!(any(target_os = "netbsd", target_os = "openbsd")) && colored_output {
                    // -G is not available in NetBSD's and OpenBSD's ls
                    cmd.push("-G");
                }

                cmd
            }
        }
    } else if cfg!(windows) {
        use std::process::{Command, Stdio};

        // Use GNU ls, if available
        let gnu_ls_exists = Command::new("ls")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok();

        if gnu_ls_exists {
            gnu_ls("ls")
        } else {
            return Err(anyhow!(
                "'fd --list-details' is not supported on Windows unless GNU 'ls' is installed."
            ));
        }
    } else {
        return Err(anyhow!(
            "'fd --list-details' is not supported on this platform."
        ));
    };
    Ok(cmd)
}

fn extract_time_constraints(opts: &Opts) -> Result<Vec<TimeFilter>> {
    let mut time_constraints: Vec<TimeFilter> = Vec::new();
    if let Some(ref t) = opts.changed_within {
        if let Some(f) = TimeFilter::after(t) {
            time_constraints.push(f);
        } else {
            return Err(anyhow!(
                "'{}' is not a valid date or duration. See 'fd --help'.",
                t
            ));
        }
    }
    if let Some(ref t) = opts.changed_before {
        if let Some(f) = TimeFilter::before(t) {
            time_constraints.push(f);
        } else {
            return Err(anyhow!(
                "'{}' is not a valid date or duration. See 'fd --help'.",
                t
            ));
        }
    }
    Ok(time_constraints)
}

fn ensure_use_hidden_option_for_leading_dot_pattern(
    config: &Config,
    pattern_regexps: &[String],
) -> Result<()> {
    if cfg!(unix)
        && config.ignore_hidden
        && pattern_regexps
            .iter()
            .any(|pat| pattern_matches_strings_with_leading_dot(pat))
    {
        Err(anyhow!(
            "The pattern(s) seems to only match files with a leading dot, but hidden files are \
            filtered by default. Consider adding -H/--hidden to search hidden files as well \
            or adjust your search pattern(s)."
        ))
    } else {
        Ok(())
    }
}

fn build_regex(pattern_regex: String, config: &Config) -> Result<regex::bytes::Regex> {
    RegexBuilder::new(&pattern_regex)
        .case_insensitive(!config.case_sensitive)
        .dot_matches_new_line(true)
        .build()
        .map_err(|e| {
            anyhow!(
                "{}\n\nNote: You can use the '--fixed-strings' option to search for a \
                 literal string instead of a regular expression. Alternatively, you can \
                 also use the '--glob' option to match on a glob pattern.",
                e.to_string()
            )
        })
}

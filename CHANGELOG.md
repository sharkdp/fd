 10.3.0

## Features

- Add a hidden `--mindepth` alias for `--min-depth`. (#1617)


## Bugfixes


## Changes

- Replace `humantime` crate and `chrono` crate with `jiff` crate, see #1690 (@sorairolake). This has some small changes to the
  way dates given to options such `--changed-within` and `--changed-before` including:
  - 'M' no longer means "month", as that could be confusing with minutes. Use "mo", "mos", "month" or "months" instead.
  - month and year now account for variability in the calander rather than being a hard-coded number of seconds. That is probably
    what you would expect, but it is a slight change in behavior.
- aarch64 Windows was added to CI and release artifacts
- Many dependencies were updated
- Better support building on Illumos (there is no automated testing, but some known issues were fixed)

## Other

This will be the last release that has been tested on x86_64 Mac OS, since GitHub is
dropping support for runners with that hardware.

It may also be the last release to use a version of Rust with tier-1 support for
x86_64/intel Macs and Windows 7.


# 10.2.0

## Features

- Add --hyperlink option to add OSC 8 hyperlinks to output


## Bugfixes


## Changes

- Build windows releases with rust 1.77 so windows 7 is still supported
- Deb packages now include symlink for fdfind to be more consistent with official packages


## Other

# 10.1.0

## Features

- Allow passing an optional argument to `--strip-cwd-prefix` of "always", "never", or "auto". to force whether the cwd prefix is stripped or not.
- Add a `--format` option which allows using a format template for direct ouput similar to the template used for `--exec`. (#1043)

## Bugfixes
- Fix aarch64 page size again. This time it should actually work. (#1085, #1549) (@tavianator)


## Other

- aarch64-apple-darwin target added to builds on the release page. Note that this is a tier 2 rust target.

# v10.0.0

## Features

- Add `dir` as an alias to `directory` when using `-t` \ `--type`, see #1460 and #1464 (@Ato2207).
- Add support for @%s date format in time filters similar to GNU date (seconds since Unix epoch for --older/--newer), see #1493 (@nabellows)
- Breaking: No longer automatically ignore `.git` when using `--hidden` with vcs ignore enabled. This reverts the change in v9.0.0. While this feature
  was often useful, it also broke some existing workflows, and there wasn't a good way to opt out of it. And there isn't really a good way for us to add
  a way to opt out of it. And you can easily get similar behavior by adding `.git/` to your global fdignore file.
    See #1457.

## Bugfixes

- Respect NO_COLOR environment variable with `--list-details` option. (#1455)
- Fix bug that would cause hidden files to be included despite gitignore rules
  if search path is "." (#1461, BurntSushi/ripgrep#2711).
- aarch64 builds now use 64k page sizes with jemalloc. This fixes issues on some systems, such as ARM Macs that
  have a larger system page size than the system that the binary was built on. (#1547)
- Address [CVE-2024-24576](https://blog.rust-lang.org/2024/04/09/cve-2024-24576.html), by increasing minimum rust version.


## Changes
- Minimum supported rust version is now 1.77.2


# v9.0.0

## Performance

- Performance has been *significantly improved*, both due to optimizations in the underlying `ignore`
  crate (#1429), and in `fd` itself (#1422, #1408, #1362) - @tavianator.
  [Benchmarks results](https://gist.github.com/tavianator/32edbe052f33ef60570cf5456b59de81) show gains
  of 6-8x for full traversals of smaller directories (100k files) and up to 13x for larger directories (1M files).

- The default number of threads is now constrained to be at most 64. This should improve startup time on
  systems with many CPU cores. (#1203, #1410, #1412, #1431) - @tmccombs and @tavianator

- New flushing behavior when writing output to stdout, providing better performance for TTY and non-TTY
  use cases, see #1452 and #1313 (@tavianator).

## Features

- Support character and block device file types, see #1213 and #1336 (@cgzones)
- Breaking: `.git/` is now ignored by default when using `--hidden` / `-H`, use `--no-ignore` / `-I` or
  `--no-ignore-vcs` to override, see #1387 and #1396 (@skoriop)

## Bugfixes

- Fix `NO_COLOR` support, see #1421 (@acuteenvy)

## Other

- Fixed documentation typos, see #1409 (@marcospb19)

## Thanks

Special thanks to @tavianator for his incredible work on performance in the `ignore` crate and `fd` itself.



# v8.7.1

## Bugfixes

- `-1` properly conflicts with the exec family of options.
- `--max-results` overrides `-1`
- `--quiet` properly conflicts with the exec family of options. This used to be the case, but broke during the switch to clap-derive
- `--changed-within` now accepts a space as well as a "T" as the separator between date and time (due to update of chrono dependency)

## Other
- Many dependencies were updated
- Some documentation was updated and fixed

# v8.7.0

## Features

- Add flag --no-require-git to always respect gitignore files, see #1216 (@vegerot)

## Bugfixes

- Fix logic for when to use global ignore file. There was a bug where the only case where the
  global ignore file wasn't processed was if `--no-ignore` was passed, but neither `--unrestricted`
  nor `--no-global-ignore-file` is passed. See #1209

# v8.6.0

## Features

- New `--and <pattern>` option to add additional patterns that must also be matched. See #315
  and #1139 (@Uthar)
- Added `--changed-after` as alias for `--changed-within`, to have a name consistent with `--changed-before`.


## Changes

- Breaking: On Unix-like systems, `--type executable` now additionally checks if
  the file is executable by the current user, see #1106 and #1169 (@ptipiak)


## Bugfixes

- Use fd instead of fd.exe for Powershell completions (when completions are generated on windows)


## Other


# v8.5.3

## Bugfixes

- Fix completion generation to not include full path of fd command
- Fix build error if completions feature is disabled

# v8.5.2

## Bugfixes

- Fix --owner option value parsing, see #1163 and #1164 (@tmccombs)


# v8.5.1

## Bugfixes

- Fix --threads/-j option value parsing, see #1160 and #1162 (@sharkdp)


# v8.5.0

## Features

- `--type executable`/`-t` now works on Windows, see #1051 and #1061 (@tavianator)

## Bugfixes

- Fixed differences between piped / non-piped output. This changes `fd`s behavior back to what we
  had before 8.3.0, i.e. there will be no leading `./` prefixes, unless `--exec`/`-x`,
  `--exec-batch`/`-X`, or `--print0`/`-0` are used. `--strip-cwd-prefix` can be used to strip that
  prefix in those cases. See #1046, #1115, and #1121 (@tavianator)
- `fd` could previously crash with a panic due to a race condition in Rusts standard library
  (see https://github.com/rust-lang/rust/issues/39364). This has been fixed by switching to a different
  message passing implementation, see #1060 and #1146 (@tavianator)
- `fd`s memory usage will not grow unboundedly on huge directory trees, see #1146 (@tavianator)
- fd returns an error when current working directory does not exist while a search path is
  specified, see #1072 (@vijfhoek)
- Improved "command not found" error message, see #1083 and #1109 (@themkat)
- Preserve command exit codes when using `--exec-batch`, see #1136 and #1137 (@amesgen)

## Changes

- No leading `./` prefix for non-interactive results, see above.
- fd now colorizes paths in parallel, significantly improving performance, see #1148 (@tavianator)
- fd can now avoid `stat` syscalls even when colorizing paths, as long as the color scheme doesn't
  require metadata, see #1148 (@tavianator)
- The statically linked `musl` versions of `fd` now use `jmalloc`, leading to a significant performance
  improvement, see #1062 (@tavianator)

## Other

- Added link back to GitHub in man page and `--help` text, see #1086 (@scottchiefbaker)
- Major update in how `fd` handles command line options internally, see #1067 (@tmccombs)

# v8.4.0

## Features

- Support multiple `--exec <cmd>` instances, see #406 and #960 (@tmccombs)

## Bugfixes

- "Argument list too long" errors can not appear anymore when using `--exec-batch`/`-X`, as the command invocations are automatically batched at the maximum possible size, even if `--batch-size` is not given. See #410 and #1020 (@tavianator)

## Changes

- Directories are now printed with an additional path separator at the end: `foo/bar/`, see #436 and #812 (@yyogo)
- The `-u` flag was changed to be equivalent to `-HI` (previously, a single `-u` was only equivalent to `-I`). Additional `-u` flags are still allowed, but ignored. See #840 and #986 (@jacksontheel)

## Other

- Added installation instructions for RHEL8, see #989 (@ethsol)


# v8.3.2

## Bugfixes

- Invalid absolute path on windows when searching from the drive root, see #931 and #936 (@gbarta)


# v8.3.1

## Bugfixes

- Stop implying `--no-ignore-parent` when `--no-vcs-ignore` is supplied, see #907, #901, #908 (@tmccombs)
- fd no longer waits for the whole traversal if the only matches arrive within max_buffer_time, see #868 and #895 (@tavianator)
- `--max-results=1` now immediately quits after the first result, see #867
- `fd -h` does not panic anymore when stdout is closed, see #897

## Changes

- Disable jemalloc on FreeBSD, see #896 (@xanderio)
- Updated man page, see #912 (@rlue)
- Updated zsh completions, see #932 (@tmccombs)


# v8.3.0

## Performance improvements

- Colorized output is now significantly faster, see #720 and #853 (@tavianator)
- Writing to stdout is now buffered if the output does not go to a TTY. This increases performance
  when the output of `fd` is piped to another program or to a file, see #885 (@tmccombs, original
  implementation by @sourlemon207)
- File metadata is now cached between the different filters that require it (e.g. `--owner`,
  `--size`), reducing the number of `stat` syscalls when multiple filters are used; see #863
  (@tavianator, original implementation by @alexmaco)

## Features

- Don't buffer command output from `--exec` when using a single thread. See #522
- Add new `-q, --quiet` flag, see #303 (@Asha20)
- Add new `--no-ignore-parent` flag, see #787 (@will459)
- Add new `--batch-size` flag, see #410 (@devonhollowood)
- Add opposing command-line options, see #595 (@Asha20)
- Add support for more filesystem indicators in `LS_COLORS`, see
  https://github.com/sharkdp/lscolors/pull/35 (@tavianator)

## Bugfixes

- Always show the `./` prefix for search results unless the output is a TTY or `--strip-cwd-prefix` is set, see #760 and #861 (@jcaplan)
- Set default path separator to `/` in MSYS, see #537 and #730 (@aswild)
- fd cannot search files under a RAM disk, see #752
- fd doesn't show substituted drive on Windows, see #365
- Properly handle write errors to devices that are full, see #737
- Use local time zone for time functions (`--change-newer-than`, `--change-older-than`), see #631 (@jacobmischka)
- Support `--list-details` on more platforms (like BusyBox), see #783
- The filters `--owner`, `--size`, and `--changed-{within,before}` now apply to symbolic links
  themselves, rather than the link target, except when `--follow` is specified; see #863
- Change time comparisons to be exclusive, see #794 (@jacobmischka)

## Changes

- Apply custom `--path-separator` to commands run with `--exec(-batch)` and `--list-details`, see #697 (@aswild)

## Other

- Many documentation updates


# v8.2.1

No functional changes with respect to v8.2.0. Bugfix in the release process.

# v8.2.0

## Features

- Add new `--prune` flag, see #535 (@reima)
- Improved the usability of the time-based options, see #624 and #645 (@gorogoroumaru)
- Add support for exact file sizes in the `--size` filter, see #669 and #696 (@Rogach)
- `fd` now prints an error message if the search pattern requires a leading dot but
  `--hidden` is not enabled (Unix only), see #615

## Bugfixes

- Avoid panic when performing limited searches in directories with restricted permissions, see #678
- Invalid numeric command-line arguments are silently ignored, see #675
- Disable jemalloc on Android, see #662
- The `--help` text will be colorless if `NO_COLOR` has been set, see #600 (@xanonid)

## Changes

- If `LS_COLORS` is not set (e.g. on Windows), we now provide a more comprehensive default which
  includes much more filetypes, see #604 and #682 (mjsir911).

## Other

- Added `zsh` completion files, see #654 and #189 (@smancill)

# v8.1.1

## Bugfixes

- Support colored output on older Windows versions if either (1) `--color=always` is set or (2) the `TERM` environment variable is set. See #469

# v8.1.0

## Features

- Add new `--owner [user][:group]` filter. See #307 (pull #581) (@alexmaco)
- Add support for a global ignore file (`~/.config/fd/ignore` on Unix), see #575 (@soedirgo)
- Do not exit immediately if one of the search paths is missing, see #587 (@DJRHails)

## Bugfixes

- Reverted a change from fd 8.0 that enabled colors on all Windows terminals (see below) in order to support older Windows versions again, see #577. Unfortunately, this re-opens #469
- Fix segfault caused by jemalloc on macOS Catalina, see #498
- Fix `--glob` behavior with empty pattern, see #579 (@SeamusConnor)
- Fix `--list-details` on FreeBSD, DragonFly BSD, OpenBSD and NetBSD. See #573 (@t6)

## Changes

- Updated documentation for `--size`, see #584

# v8.0.0

## Features

- Add a new `-l`/`--list-details` option to show more details about the search results. This is
  basically an alias for `--exec-batch ls -l` with some additional `ls` options.
  This can be used in order to:
    * see metadata like permissions, owner, file size, modification times (#491)
    * see symlink targets (#482)
    * achieve a deterministic output order (#324, #196, #159)
- Add a new `--max-results=<count>` option to limit the number of search results, see #472, #476 and #555
  This can be useful to speed up searches in cases where you know that there are only N results.
  Using this option is also (slightly) faster than piping to `head -n <count>` where `fd` can only
  exit when it finds the search results `<count> + 1`.
- Add the alias `-1` for `--max-results=1`, see #561. (@SimplyDanny).
- Add new `--type socket` and `--type pipe` filters, see #511.
- Add new `--min-depth <depth>` and `--exact-depth <depth>` options in addition to the existing option
  to limit the maximum depth. See #404.
- Support additional ANSI font styles in `LS_COLORS`: faint, slow blink, rapid blink, dimmed, hidden and strikethrough.

## Bugfixes

- Preserve non-UTF8 filenames: invalid UTF-8 filenames are now properly passed to child-processes
  when using `--exec`, `--exec-batch` or `--list-details`. In `fd`'s output, we replace non-UTF-8
  sequences with the "�" character. However, if the output of `fd` goes to another process, we
  print the actual bytes of the filename. For more details, see #558 and #295.
- `LS_COLORS` entries with unsupported font styles are not completely ignored, see #552

## Changes

- Colored output will now be enabled by default on older Windows versions.
  This allows the use of colored output if the terminal supports it (e.g.
  MinTTY, Git Bash). On the other hand, this will be a regression for users
  on older Windows versions with terminals that do not support ANSI escape
  sequences. Affected users can use an alias `fd="fd --color=never"` to
  continue using `fd` without colors. There is no change of behavior for
  Windows 10. See #469.
- When using `--glob` in combination with `--full-path`, a `*` character does not match a path
  separation character (`/` or `\\`) anymore. You can use `**` for that. This allows things like
  `fd -p -g '/some/base/path/*/*/*.txt'` which would previously match to arbitrary depths (instead
  of exactly two folders below `/some/base/path`. See #404.
- "Legacy" support to use `fd -exec` (with a single dash) has been removed. Use `fd -x` or
  `fd --exec` instead.
- Overall improved error handling and error messages.


## Other

- Korean translation of the README, see: [한국어](https://github.com/spearkkk/fd-kor) (@spearkkk)


# v7.5.0

## Features

- Added `--one-file-system` (aliases: `--mount`, `--xdev`) to not cross file system boundaries on Unix and Windows, see #507 (@FallenWarrior2k).
- Added `--base-directory` to change the working directory in which `fd` is run, see #509 and #475 (@hajdamak).
- `fd` will not use colored output if the `NO_COLOR` environment variable is set, see #550 and #551 (@metadave).
- `fd --exec` will return exit code 1 if one of the executed commands fails, see #526 and #531 (@fusillicode and @Giuffre)

## Bug Fixes

- Fixed 'command not found' error when using zsh completion, see #487 (@barskern).
- `fd -L` should include broken symlinks, see #357 and #497 (@tommilligan, @neersighted and @sharkdp)
- Display directories even if we don't have permission to enter, see #437 (@sharkdp)

## Changes

- A flag can now be passed multiple times without producing an error, see #488 and #496 (@rootbid).
- Search results are sorted when using the `-X` option to match the behaviour of piping to `xargs`, see #441 and #524 (@Marcoleni @crash-g).


# v7.4.0

## Performance improvements

- Reduce number of `stat` syscalls, improving the performance for searches where file metadata is
  required (`--type`, `--size`, `--changed-within`, …), see #434 (@tavianator)
- Use jemalloc by default, improving the performance for almost all searches, see #481. Note that
  Windows and `*musl*` builds do not profit from this.

## Features

- Added a new `-g`/`--glob` option to switch to glob-based searches (instead of regular expression
  based searches). This is accompanied by a new `--regex` option that can be used to switch back,
  if users want to `alias fd="fd --glob"`. See #284
- Added a new `--path-separator <sep>` option which can be useful for Windows users who
  want/need `fd` to use `/` instead of `\`, see #428 and #153 (@mookid)
- Added support for hidden files on Windows, see #379
- When `fd` is run with the `--exec-batch`/`-X` option, it now exposes the exit status of the
  command that was run, see #333.
- Exit immediately when Ctrl-C has been pressed twice, see #423

## Bugfixes

- Make `--changed-within`/`--changed-before` work for directories, see #470

## Other

- Pre-built `fd` binaries should now be available for `armhf` targets, see #457 (@detly)
- `fd` is now available on Alpine Linux, see #451 (@5paceToast)
- `fd` is now in the officla FreeBSD repositories, see #412 (@t6)
- Added OpenBSD install instructions, see #421 (@evitalis)
- Added metadata to the Debian package, see #416 (@cathalgarvey)
- `fd` can be installed via npm, see #438 (@pablopunk)


# v7.3.0

## Features

- New `--exec-batch <cmd>`/`-X <cmd>` option for batch execution of commands, see #360 (@kimsnj).
  This allows you to do things like:
  ``` bash
  fd … -X vim  # open all search results in vim (or any other editor)
  fd … -X ls -l  # view detailed stats about the search results with 'ls'
  fd -e svg -X inkscape  # open all SVG files in Inkscape
  ```
- Support for 24-bit color codes (when specified via `LS_COLORS`) as well as
  different font styles (bold, italic, underline).

## Changes

- A few performance improvements, in particular when printing lots of colorized
  results to the console, see #370
- The `LS_COLORS` handling has been "outsourced" to a separate crate (https://github.com/sharkdp/lscolors) that is now being used by other tools as well: [fselect](https://github.com/jhspetersson/fselect), [lsd](https://github.com/Peltoche/lsd/pull/84). For details, see #363.

## Other

- `fd` will be available in Ubuntu Disco DIngo (19.04), see #373 (@sylvestre)
- This release should come with a static ARM binary (`arm-unknown-linux-musleabihf`), see #320 (@duncanfinney)
- Various documentation improvements, see #389

## Thanks

Special thanks to @alexmaco for his awesome work on refactoring and code improvements! (see #401, #398, and #383)

# v7.2.0

## Features

* Added support for filtering by file modification time by adding two new options `--changed-before <date|duration>` and `--changed-within <..>`. For more details, see the `--help` text, the man page, the relevant issue #165 and the PR #339 (@kimsnj)
* Added `--show-errors` option to enable the display of filesystem error messages such as "permission denied", see #311 (@psinghal20 and @majecty)
* Added `--maxdepth` as a (hidden) alias for `--max-depth`, see #323 (@mqudsi)
* Added `--search-path` option which can be supplied to replace the positional `path` argument at any position.

## Changes

* Loosen strict handling of missing `--ignore-file`, see #280 (@psinghal20)
* Re-enabled `.ignore` files, see #156.

## Bugfixes

* `fd` could previously get stuck when run from the root directory in the
  presence of zombie processes. This curious bug has been fixed in Rust 1.29 and higher. For more details, see #288, [rust-lang/rust#50619](https://github.com/rust-lang/rust/issues/50619) and [the fix](https://github.com/rust-lang/rust/pull/50630)

## Other

* `fd` has officially landed in Debian! See #345 for details. Thanks goes to @sylvestre, @paride and possibly others I don't know about.
* Added Chinese translation of README (@chinanf-boy)

## Thanks

A special thanks goes to @joshleeb for his amazing improvements throughout
the code base (new tests, refactoring work and various other things)!


# v7.1.0

## Features

* Added `--size` filter option, see #276 (@stevepentland, @JonathanxD and @alexmaco)
* Added `--type empty` (or `-t e`) to search for empty files and/or directories, see #273

## Changes

* With the new version, `.gitignore` files will only be respected in Git repositories, not outside.
* A few performance improvements for `--type` searches, see 641976cf7ad311ba741571ca8b7f02b2654b6955 and 50a2bab5cd52d26d4a3bc786885a2c270ed3b227

## Other

* Starting with this release, we will offer pre-built ARM binaries, see #244
* Added instructions on how to use `fd` with `emacs`, see #282 (@redguardtoo)
* `fd` is now in the official openSUSE repositories, see #275 (@avindra)
* `fd` is now available via MacPorts, see #291 (@raimue)


# v7.0.0

## Features

* Added `--type executable` (or `-t x`) to search for executable files only, see #246 (@PramodBisht)
* Added support for `.fdignore` files, see #156 and #241.
* Added `--ignore-file` option to add custom ignore files, see #156.
* Suggest `--fixed-strings` on invalid regular expressions, see #234 (@PramodBisht)
* Detect when user supplied path instead of pattern, see #235.

## Changes

* `.ignore` and `.rgignore` files are not parsed anymore. Use `.fdignore` files
  or add custom files via `--ignore-file` instead.
* Updated to `regex-syntax` 0.5 (@cuviper)

## Bugfixes

* Properly normalize absolute paths, see #268
* Invalid utf8 filenames displayed when `-e` is used, see #250
* If `--type` is used, fifos/sockets/etc. are always shown, see #260

## Other

* Packaging:
    * The Arch Linux package is now simply called `fd`.
    * There is now a `fd` ebuild for Gentoo Linux.
    * There is a `scoop` package for `fd` (Windows).
    * There is a `Chocolatey` package for `fd` (Windows).
    * There is a Fedora `copr` package for `fd`.


# v6.3.0

## Features

* Files with multiple extensions can now be found via `--extension`/`-e`, see #214 (@althonos)
  ``` bash
  > fd -e tar.gz
  ```

* Added new `-F`/`--fixed-strings`/`--literal` option that treats the pattern as a literal string instead of a regular expression, see #157

  ``` bash
  > fd -F 'file(1).txt'
  ```

* Allow `-exec` to work as `--exec`, see #226 (@stevepentland)

## Bugfixes

* Fixed `Ctrl-C` handling when using `--exec`, see #224 (@Doxterpepper)

* Fixed wrong file owner for files in deb package, see #213

## Other

* Replaced old gif by a fancy new SVG screencast (@marionebl)
* Updated [benchmark results](https://github.com/sharkdp/fd#benchmark) (fd has become faster in the meantime!). There is a new repository that hosts several benchmarking scripts for fd: https://github.com/sharkdp/fd-benchmarks


# v6.2.0

## Features

* Support for filtering by multiple file extensions and multiple file types, see #199 and #177
  (@tkadur).

  For example, it's possible to search for C++ source or header files:
  ``` bash
  > fd -e cpp -e c -e cxx -e h pattern
  ```

## Changes

* The size of the output buffer (for sorting search results) is now limited to 1000 entries. This
  improves the search speed significantly if there are a lot of results, see #191 (@sharkdp).

## Bugfixes

* Fix a bug where long-running searches could not be killed via Ctrl-C, see #210 (@Doxterpepper)
* fd's exit codes are now in accordance with Unix standards, see #201 (@Doxterpepper)

## Other

* Bash, zsh and fish completion should now work with the Ubuntu `.deb` packages, see #195 and #209
  (@tmccombs and @sharkdp)
* There is a new section on how to set up `fzf` to use `fd` in the
  [README](https://github.com/sharkdp/fd#using-fd-with-fzf), see #168.


# v6.1.0

## Features

* Support for multiple search paths, see #166 (@Doxterpepper)
* Added `--no-ignore-vcs` option to disable `.gitignore` and other VCS ignore files,
  without disabling `.ignore` files - see #156 (@ptzz).

## Bugfixes

* Handle terminal signals, see #128 (@Doxterpepper)
* Fixed hang on `--exec` when user input was required, see #178 and #193 (@reima)

## Other

* Debian packages are now created via Travis CI and should be available for this and all
  future releases (@tmccombs).
* fd is now available on Void Linux (@maxice8)
* The minimum required Rust version is now 1.20

## Thanks

@Doxterpepper deserves a special mention for his great work that is included in this release and
for the support in ticket discussions and concerning Travis CI fixes. Thank you very much!

Thanks also go out to @tmccombs for the work on Debian packages and for reviewing a lot of pull requests!

# v6.0.0

## Changes

- The `--exec`/`-x` option does not spawn an intermediate shell anymore. This improves the
  performance of parallel command execution and fixes a whole class of (present and potentially
  future) problems with shell escaping. The drawback is that shell commands cannot directly be
  called with `--exec`. See #155 for the full discussion. These changes have been implemented by
  @reima (Thanks!).

## Bugfixes

- `--exec` does not escape cmd.exe metacharacters on Windows (see #155, as above).

## Other

* *fd* is now available in the FreeBSD ports (@andoriyu)
* The minimal `rustc` version is now checked when building with `cargo`, see #164 (@matematikaadit)
* The output directory for the shell completion files is created if it does not exist (@andoriyu)


# v5.0.0

## Features

* Added new `--exec`, `-x` option for parallel command execution (@mmstick, see #84 and #116). See the corresponding [README section](https://github.com/sharkdp/fd#parallel-command-execution) for an introduction.
* Auto-disable color output on unsupported Windows shells like `cmd.exe` (@iology, see #129)
* Added the `--exclude`, `-X` option to suppress certain files/directories in the search results
  (see #89).
* Added ripgrep aliases `-u` and `-uu` for `--no-ignore` and `--no-ignore --hidden`, respectively
  (@unsignedint, see #92)
* Added `-i`, `--ignore-case` (@iology, see #95)
* Made smart case really smart (@reima, see #103)
* Added RedoxOS support (@goyox86, see #131)

## Changes

* The dot `.` can now match newlines in file names (@iology, see #111)
* The short `--type` argument for symlinks has been changed from `s` to `l` (@jcpetkovich, see #83)

## Bugfixes

* Various improvements in root-path and symlink handling (@iology, see #82, #107, and #113)
* Fixed absolute path handling on Windows (@reima, #93)
* Fixed: current directory not included when using relative path (see #81)
* Fixed `--type` behavior for unknown file types (@iology, see #150)
* Some fixes around `--exec` (@iology, see #142)

## Other

* Major updates and bugfixes to our continuous integration and deployment tooling on Travis
  (@matematikaadit, see #149, #145, #133)
* Code style improvements & automatic style checking via `rustfmt` on Travis (@Detegr, see #99)
* Added a man page (@pickfire, see #77)
* *fd* has been relicensed under the dual license MIT/Apache-2.0 (@Detegr, see #105)
* Major refactorings and code improvements (Big thanks to @gsquire, @reima, @iology)
* First version of [`CONTRIBUTING`](https://github.com/sharkdp/fd/blob/master/CONTRIBUTING.md) guidelines
* There is now a Nix package (@mehandes)
* *fd* is now in the official Arch Linux repos (@cassava)
* Improved tooling around shell completion files (@ImbaKnugel, see #124)
* Updated tutorial in the [`README`](https://github.com/sharkdp/fd/blob/master/README.md)
* The minimum required version of Rust has been bumped to 1.19.

## Thanks

A *lot* of things have happened since the last release and I'd like to thank all contributors for their great support. I'd also like to thank those that have contributed by reporting bugs and by posting feature requests.

I'd also like to take this chance to say a special Thank You to a few people that have stood out in one way or another: To @iology, for contributing a multitude of bugfixes, improvements and new features. To @reima and @Detegr for their continuing great support. To @mmstick, for implementing the most advanced new feature of *fd*. And to @matematikaadit for the CI/tooling upgrades.


# v4.0.0

## Features

* Added filtering by file extension, for example `fd -e txt`, see #56 (@reima)
* Add option to force colored output: `--color always`, see #49 (@Detegr)
* Generate Shell completions for Bash, ZSH, Fish and Powershell, see #64 (@ImbaKnugel)
* Better & extended `--help` text (@abaez and @Detegr)
* Proper Windows support, see #70

## Changes

* The integration tests have been re-written in Rust :sparkles:, making them platform-independent and easily callable via `cargo test` - see #65  (many thanks to @reima!)
* New tutorial in the README (@deg4uss3r)
* Reduced number of `stat` syscalls for each result from 3 to 1, see #36.
* Enabled Appveyor CI

# v3.1.0

## Features
- Added file type filtering, e.g. `find --type directory` or `find -t f` (@exitium)

# v3.0.0

## Features
- Directories are now traversed in parallel, leading to significant performance improvements (see [benchmarks](https://github.com/sharkdp/fd#benchmark))
- Added `--print0` option (@michaelmior)
- Added AUR packages (@wezm)

## Changes
- Changed short flag for `--follow` from `-f` to `-L` (consistency with `ripgrep`)

# v2.0.0

* Changed `--sensitive` to `--case-sensitive`
* Changed `--absolute` to `--absolute-path`
* Throw an error if root directory is not existent, see #39
* Use absolute paths if the root dir is an absolute path, see #40
* Handle invalid UTF-8, see #34 #38
* Support `-V`, `--version` by switching from `getopts` to `clap`.

Misc:
* It's now possible to install `fd` via homebrew on macOS: `brew install fd`.

# v1.1.0

- Windows compatibility (@sebasv), see #29 #35
- Safely exit on broken output pipes (e.g.: usage with `head`, `tail`, ..), see #24
- Backport for rust 1.16, see #23

# v1.0.0

* Respect `.(git)ignore` files
* Use `LS_COLORS` environment variable directly, instead of `~/.dir_colors` file.
* Added unit and integration tests
* Added optional second argument (search path)

# v0.3.0

-  Parse dircolors files, closes #20
-  Colorize each path component, closes #19
-  Add short command line option for --hidden, see #18

# v0.2.0

-  Option to follow symlinks, disable colors, closes #16, closes #17
- `--filename` instead of `--full-path`
-  Option to search hidden directories, closes #12
-  Configurable search depth, closes #13
-  Detect interactive terminal, closes #11

# v0.1.0

Initial release

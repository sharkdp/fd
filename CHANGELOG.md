# Upcoming release

## Features

- Add a new `-l`/`--list-details` option to show more details about the search results. This is
  basically an alias for `--exec-batch ls -l` with some additional `ls` options.
  This can be used in order to:
    * see metadata like permissions, owner, file size, modification times (#491)
    * see symlink targets (#482)
    * achieve a deterministic output order (#324, #196, #159)
- Add a new `--max-results=<count>` option to limit the number of search results, see #472 and #476
  This can be useful to speed up searches in cases where you know that there are only N results.
  Using this option is also (slightly) faster than piping to `head -n <count>` where `fd` can only
  exit when it finds the search results `<count> + 1`.
- Add the alias `-1` for `--max-results=1`, see #561. (@SimplyDanny).
- Support additional ANSI font styles in `LS_COLORS`: faint, slow blink, rapid blink, dimmed, hidden and strikethrough.

## Bugfixes

- `LS_COLORS` entries with unsupported font styles are not completely ignored, see #552

## Changes



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

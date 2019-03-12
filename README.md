# fd
[![Build Status](https://travis-ci.org/sharkdp/fd.svg?branch=master)](https://travis-ci.org/sharkdp/fd)
[![Build status](https://ci.appveyor.com/api/projects/status/21c4p5fwggc5gy3j?svg=true)](https://ci.appveyor.com/project/sharkdp/fd)
[![Version info](https://img.shields.io/crates/v/fd-find.svg)](https://crates.io/crates/fd-find)
[中文](https://github.com/chinanf-boy/fd-zh)

*fd* is a simple, fast and user-friendly alternative to
[*find*](https://www.gnu.org/software/findutils/).

While it does not seek to mirror all of *find*'s powerful functionality, it provides sensible
(opinionated) defaults for [80%](https://en.wikipedia.org/wiki/Pareto_principle) of the use cases.

## Features
* Convenient syntax: `fd PATTERN` instead of `find -iname '*PATTERN*'`.
* Colorized terminal output (similar to *ls*).
* It's *fast* (see [benchmarks](#benchmark) below).
* Smart case: the search is case-insensitive by default. It switches to
  case-sensitive if the pattern contains an uppercase
  character[\*](http://vimdoc.sourceforge.net/htmldoc/options.html#'smartcase').
* Ignores hidden directories and files, by default.
* Ignores patterns from your `.gitignore`, by default.
* Regular expressions.
* Unicode-awareness.
* The command name is *50%* shorter[\*](https://github.com/ggreer/the_silver_searcher) than
  `find` :-).
* Parallel command execution with a syntax similar to GNU Parallel.

## Demo

![Demo](doc/screencast.svg)

## Benchmark

Let's search my home folder for files that end in `[0-9].jpg`. It contains ~190.000
subdirectories and about a million files. For averaging and statistical analysis, I'm using
[hyperfine](https://github.com/sharkdp/hyperfine). The following benchmarks are performed
with a "warm"/pre-filled disk-cache (results for a "cold" disk-cache show the same trends).

Let's start with `find`:
```
Benchmark #1: find ~ -iregex '.*[0-9]\.jpg$'

  Time (mean ± σ):      7.236 s ±  0.090 s
 
  Range (min … max):    7.133 s …  7.385 s
```

`find` is much faster if it does not need to perform a regular-expression search:
```
Benchmark #2: find ~ -iname '*[0-9].jpg'

  Time (mean ± σ):      3.914 s ±  0.027 s
 
  Range (min … max):    3.876 s …  3.964 s
```

Now let's try the same for `fd`. Note that `fd` *always* performs a regular expression
search. The options `--hidden` and `--no-ignore` are needed for a fair comparison,
otherwise `fd` does not have to traverse hidden folders and ignored paths (see below):
```
Benchmark #3: fd -HI '.*[0-9]\.jpg$' ~

  Time (mean ± σ):     811.6 ms ±  26.9 ms
 
  Range (min … max):   786.0 ms … 870.7 ms
```
For this particular example, `fd` is approximately nine times faster than `find -iregex`
and about five times faster than `find -iname`. By the way, both tools found the exact
same 20880 files :smile:.

Finally, let's run `fd` without `--hidden` and `--no-ignore` (this can lead to different
search results, of course). If *fd* does not have to traverse the hidden and git-ignored
folders, it is almost an order of magnitude faster:
```
Benchmark #4: fd '[0-9]\.jpg$' ~

  Time (mean ± σ):     123.7 ms ±   6.0 ms
 
  Range (min … max):   118.8 ms … 140.0 ms
```

**Note**: This is *one particular* benchmark on *one particular* machine. While I have
performed quite a lot of different tests (and found consistent results), things might
be different for you! I encourage everyone to try it out on their own. See 
[this repository](https://github.com/sharkdp/fd-benchmarks) for all necessary scripts.

Concerning *fd*'s speed, the main credit goes to the `regex` and `ignore` crates that are also used
in [ripgrep](https://github.com/BurntSushi/ripgrep) (check it out!).

## Colorized output
`fd` can colorize files by extension, just like `ls`. In order for this to work, the environment
variable [`LS_COLORS`](https://linux.die.net/man/5/dir_colors) has to be set. Typically, the value
of this variable is set by the `dircolors` command which provides a convenient configuration format
to define colors for different file formats.
On most distributions, `LS_COLORS` should be set already. If you are looking for alternative, more
complete (and more colorful) variants, see
[here](https://github.com/seebi/dircolors-solarized) or
[here](https://github.com/trapd00r/LS_COLORS).

## Parallel command execution
If the `-x`/`--exec` option is specified alongside a command template, a job pool will be created
for executing commands in parallel for each discovered path as the input. The syntax for generating
commands is similar to that of GNU Parallel:

- `{}`: A placeholder token that will be replaced with the path of the search result
  (`documents/images/party.jpg`).
- `{.}`: Like `{}`, but without the file extension (`documents/images/party`).
- `{/}`: A placeholder that will be replaced by the basename of the search result (`party.jpg`).
- `{//}`: Uses the parent of the discovered path (`documents/images`).
- `{/.}`: Uses the basename, with the extension removed (`party`).

``` bash
# Convert all jpg files to png files:
fd -e jpg -x convert {} {.}.png

# Unpack all zip files (if no placeholder is given, the path is appended):
fd -e zip -x unzip

# Convert all flac files into opus files:
fd -e flac -x ffmpeg -i {} -c:a libopus {.}.opus

# Count the number of lines in Rust files (the command template can be terminated with ';'):
fd -x wc -l \; -e rs
```

## Installation

### On Ubuntu
*... and other Debian-based Linux distributions.*

Download the latest `.deb` package from the [release page](https://github.com/sharkdp/fd/releases) and install it via:
``` bash
sudo dpkg -i fd_7.3.0_amd64.deb  # adapt version number and architecture
```

Note: `fd` will be officially available in Ubuntu Disco Dingo (19.04).

### On Debian

If you run Debian Buster or newer, you can install the
[officially maintained Debian package](https://tracker.debian.org/pkg/rust-fd-find):
```
sudo apt-get install fd-find
```
Note that the binary is called `fdfind` as the binary name `fd` is already used by another package.
It is recommended that you add an `alias fd=fdfind` to your shells initialization file, in order to
use `fd` in the same way as in this documentation.

### On Fedora

Starting with Fedora 28, you can install `fd` from the official package sources:
``` bash
dnf install fd-find
```

For older versions, you can use this [Fedora copr](https://copr.fedorainfracloud.org/coprs/keefle/fd/) to install `fd`:
``` bash
dnf copr enable keefle/fd
dnf install fd
```

### On Arch Linux

You can install [the fd package](https://www.archlinux.org/packages/community/x86_64/fd/) from the official repos:
```
pacman -S fd
```
### On Gentoo Linux

You can use [the fd ebuild](https://packages.gentoo.org/packages/sys-apps/fd) from the official repo:
```
emerge -av fd
```

### On openSUSE Linux

You can install [the fd package](https://software.opensuse.org/package/fd) from the official repo:
```
zypper in fd
```

### On Void Linux

You can install `fd` via xbps-install:
```
xbps-install -S fd
```

### On macOS

You can install `fd` with [Homebrew](http://braumeister.org/formula/fd):
```
brew install fd
```

… or with MacPorts:
```
sudo port install fd
```

### On Windows

You can download pre-built binaries from the [release page](https://github.com/sharkdp/fd/releases).

Alternatively, you can install `fd` via [Scoop](http://scoop.sh):
```
scoop install fd
```

Or via [Chocolatey](https://chocolatey.org):
```
choco install fd
```

### On NixOS / via Nix

You can use the [Nix package manager](https://nixos.org/nix/) to install `fd`:
```
nix-env -i fd
```

### On FreeBSD

You can install [the fd-find package](https://www.freshports.org/sysutils/fd) from the official repo:
```
pkg install fd-find
```

### On OpenBSD
You can install the [fd-find](https://gitlab.com/epbsd/ports/tree/master/sysutils/fd) package from the [EPBSD](https://epbsd.org/) repo:
```
ftp -o /etc/signify/epbsd-pkg.pub https://epbsd.org/epbsd-pkg.pub
export PKG_PATH=https://repo.epbsd.org/pub/OpenBSD/$(arch -s):${PKG_PATH}
pkg_add fd
```

### From source

With Rust's package manager [cargo](https://github.com/rust-lang/cargo), you can install *fd* via:
```
cargo install fd-find
```
Note that rust version *1.31.0* or later is required.

### From binaries

The [release page](https://github.com/sharkdp/fd/releases) includes precompiled binaries for Linux, macOS and Windows.

## Development
```bash
git clone https://github.com/sharkdp/fd

# Build
cd fd
cargo build

# Run unit tests and integration tests
cargo test

# Install
cargo install
```

## Command-line options
```
USAGE:
    fd [FLAGS/OPTIONS] [<pattern>] [<path>...]

FLAGS:
    -H, --hidden            Search hidden files and directories
    -I, --no-ignore         Do not respect .(git|fd)ignore files
        --no-ignore-vcs     Do not respect .gitignore files
    -s, --case-sensitive    Case-sensitive search (default: smart case)
    -i, --ignore-case       Case-insensitive search (default: smart case)
    -F, --fixed-strings     Treat the pattern as a literal string
    -a, --absolute-path     Show absolute instead of relative paths
    -L, --follow            Follow symbolic links
    -p, --full-path         Search full path (default: file-/dirname only)
    -0, --print0            Separate results by the null character
    -h, --help              Prints help information
    -V, --version           Prints version information

OPTIONS:
    -d, --max-depth <depth>            Set maximum search depth (default: none)
    -t, --type <filetype>...           Filter by type: file (f), directory (d), symlink (l),
                                       executable (x), empty (e)
    -e, --extension <ext>...           Filter by file extension
    -x, --exec <cmd>                   Execute a command for each search result
    -X, --exec-batch <cmd>             Execute a command with all search results at once
    -E, --exclude <pattern>...         Exclude entries that match the given glob pattern
    -c, --color <when>                 When to use colors: never, *auto*, always
    -S, --size <size>...               Limit results based on the size of files.
        --changed-within <date|dur>    Filter by file modification time (newer than)
        --changed-before <date|dur>    Filter by file modification time (older than)

ARGS:
    <pattern>    the search pattern, a regular expression (optional)
    <path>...    the root directory for the filesystem search (optional)
```

This is the output of `fd -h`. To see the full set of command-line options, use `fd --help` which
also includes a much more detailed help text.

## Tutorial

First, to get an overview of all available command line options, you can either run
`fd -h` for a concise help message (see above) or `fd --help` for a more detailed
version.

### Simple search

*fd* is designed to find entries in your filesystem. The most basic search you can perform is to
run *fd* with a single argument: the search pattern. For example, assume that you want to find an
old script of yours (the name included `netflix`):
``` bash
> fd netfl
Software/python/imdb-ratings/netflix-details.py
```
If called with just a single argument like this, *fd* searches the current directory recursively
for any entries that *contain* the pattern `netfl`.

### Regular expression search

The search pattern is treated as a regular expression. Here, we search for entries that start
with `x` and end with `rc`:
``` bash
> cd /etc
> fd '^x.*rc$'
X11/xinit/xinitrc
X11/xinit/xserverrc
```

### Specifying the root directory

If we want to search a specific directory, it can be given as a second argument to *fd*:
``` bash
> fd passwd /etc
/etc/default/passwd
/etc/pam.d/passwd
/etc/passwd
```

### Running *fd* without any arguments

*fd* can be called with no arguments. This is very useful to get a quick overview of all entries
in the current directory, recursively (similar to `ls -R`):
``` bash
> cd fd/tests
> fd
testenv
testenv/mod.rs
tests.rs
```

If you want to use this functionality to list all files in a given directory, you have to use
a catch-all pattern such as `.` or `^`:
``` bash
> fd . fd/tests/
testenv
testenv/mod.rs
tests.rs
```

### Searching for a particular file extension

Often, we are interested in all files of a particular type. This can be done with the `-e` (or
`--extension`) option. Here, we search for all Markdown files in the fd repository:
``` bash
> cd fd
> fd -e md
CONTRIBUTING.md
README.md
```

The `-e` option can be used in combination with a search pattern:
``` bash
> fd -e rs mod
src/fshelper/mod.rs
src/lscolors/mod.rs
tests/testenv/mod.rs
```

### Hidden and ignored files
By default, *fd* does not search hidden directories and does not show hidden files in the
search results. To disable this behavior, we can use the `-H` (or `--hidden`) option:
``` bash
> fd pre-commit
> fd -H pre-commit
.git/hooks/pre-commit.sample
```

If we work in a directory that is a Git repository (or includes Git repositories), *fd* does not
search folders (and does not show files) that match one of the `.gitignore` patterns. To disable
this behavior, we can use the `-I` (or `--no-ignore`) option:
``` bash
> fd num_cpu
> fd -I num_cpu
target/debug/deps/libnum_cpus-f5ce7ef99006aa05.rlib
```

To really search *all* files and directories, simply combine the hidden and ignore features to show
everything (`-HI`).

### Excluding specific files or directories

Sometimes we want to ignore search results from a specific subdirectory. For example, we might
want to search all hidden files and directories (`-H`) but exclude all matches from `.git`
directories. We can use the `-E` (or `--exclude`) option for this. It takes an arbitrary glob
pattern as an argument:
``` bash
> fd -H -E .git …
```

We can also use this to skip mounted directories:
``` bash
> fd -E /mnt/external-drive …
```

.. or to skip certain file types:
``` bash
> fd -E '*.bak' …
```

To make exclude-patterns like these permanent, you can create a `.fdignore` file. They work like
`.gitignore` files, but are specific to `fd`. For example:
``` bash
> cat ~/.fdignore
/mnt/external-drive
*.bak
```
Note: `fd` also supports `.ignore` files that are used by other programs such as `rg` or `ag`.

### Using fd with `xargs` or `parallel`

If we want to run a command on all search results, we can pipe the output to `xargs`:
``` bash
> fd -0 -e rs | xargs -0 wc -l
```
Here, the `-0` option tells *fd* to separate search results by the NULL character (instead of
newlines). In the same way, the `-0` option of `xargs` tells it to read the input in this way.

### Troubleshooting

#### `fd` does not find my file!

Remember that `fd` ignores hidden directories and files by default. It also ignores patterns
from `.gitignore` files. If you want to make sure to find absolutely every possible file, always
use the options `-H` and `-I` to disable these two features:
``` bash
> fd -HI …
```

#### `fd` doesn't seem to interpret my regex pattern correctly

A lot of special regex characters (like `[]`, `^`, `$`, ..) are also special characters in your
shell. If in doubt, always make sure to put single quotes around the regex pattern:

``` bash
> fd '^[A-Z][0-9]+$'
```

If your pattern starts with a dash, you have to add `--` to signal the end of command line
options. Otherwise, the pattern will be interpreted as a command-line option. Alternatively,
use a character class with a single hyphen character:

``` bash
> fd -- '-pattern'
> fd '[-]pattern'
```

### Integration with other programs

#### Using fd with `fzf`

You can use *fd* to generate input for the command-line fuzzy finder [fzf](https://github.com/junegunn/fzf):
``` bash
export FZF_DEFAULT_COMMAND='fd --type file'
export FZF_CTRL_T_COMMAND="$FZF_DEFAULT_COMMAND"
```

Then, you can type `vim <Ctrl-T>` on your terminal to open fzf and search through the fd-results.

Alternatively, you might like to follow symbolic links and include hidden files (but exclude `.git` folders):
``` bash
export FZF_DEFAULT_COMMAND='fd --type file --follow --hidden --exclude .git'
```

You can even use fd's colored output inside fzf by setting:
``` bash
export FZF_DEFAULT_COMMAND="fd --type file --color=always"
export FZF_DEFAULT_OPTS="--ansi"
```

For more details, see the [Tips section](https://github.com/junegunn/fzf#tips) of the fzf README.

#### Using fd with `emacs`

The emacs package [find-file-in-project](https://github.com/technomancy/find-file-in-project) can
use *fd* to find files.

After installing `find-file-in-project`, add the line `(setq ffip-use-rust-fd t)` to your
`~/.emacs` or `~/.emacs.d/init.el` file.

In emacs, run `M-x find-file-in-project-by-selected` to find matching files. Alternatively, run
`M-x find-file-in-project` to list all available files in the project.

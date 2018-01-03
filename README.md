# fd
[![Build Status](https://travis-ci.org/sharkdp/fd.svg?branch=master)](https://travis-ci.org/sharkdp/fd)
[![Build status](https://ci.appveyor.com/api/projects/status/21c4p5fwggc5gy3j?svg=true)](https://ci.appveyor.com/project/sharkdp/fd)
[![Version info](https://img.shields.io/crates/v/fd-find.svg)](https://crates.io/crates/fd-find)

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

![Demo](http://i.imgur.com/kTMFSVU.gif)

## Benchmark
Let's search my home folder for files that end in `[0-9].jpg`. It contains ~150.000
subdirectories and about a million files. For averaging and statistical analysis, I'm using
[bench](https://github.com/Gabriel439/bench). All benchmarks are performed for a "warm
cache". Results for a cold cache are similar.

Let's start with `find`:
```
find ~ -iregex '.*[0-9]\.jpg$'

time                 6.265 s    (6.127 s .. NaN s)
                     1.000 R²   (1.000 R² .. 1.000 R²)
mean                 6.162 s    (6.140 s .. 6.181 s)
std dev              31.73 ms   (0.0 s .. 33.48 ms)
```

`find` is much faster if it does not need to perform a regular-expression search:
```
find ~ -iname '*[0-9].jpg'

time                 2.866 s    (2.754 s .. 2.964 s)
                     1.000 R²   (0.999 R² .. 1.000 R²)
mean                 2.860 s    (2.834 s .. 2.875 s)
std dev              23.11 ms   (0.0 s .. 25.09 ms)
```

Now let's try the same for `fd`. Note that `fd` *always* performs a regular expression
search. The options `--hidden` and `--no-ignore` are needed for a fair comparison,
otherwise `fd` does not have to traverse hidden folders and ignored paths (see below):
```
fd --hidden --no-ignore '.*[0-9]\.jpg$' ~

time                 892.6 ms   (839.0 ms .. 915.4 ms)
                     0.999 R²   (0.997 R² .. 1.000 R²)
mean                 871.2 ms   (857.9 ms .. 881.3 ms)
std dev              15.50 ms   (0.0 s .. 17.49 ms)
```
For this particular example, `fd` is approximately seven times faster than `find -iregex`
and about three times faster than `find -iname`. By the way, both tools found the exact
same 14030 files :smile:.

Finally, let's run `fd` without `--hidden` and `--no-ignore` (this can lead to different
search results, of course):
```
fd '[0-9]\.jpg$' ~

time                 159.5 ms   (155.8 ms .. 165.3 ms)
                     0.999 R²   (0.996 R² .. 1.000 R²)
mean                 158.7 ms   (156.5 ms .. 161.6 ms)
std dev              3.263 ms   (2.401 ms .. 4.298 ms)
```

**Note**: This is *one particular* benchmark on *one particular* machine. While I have
performed quite a lot of different tests (and found consistent results), things might
be different for you! I encourage everyone to try it out on their own.

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
*... and other Debian-based Linux distrutions.*

Download the latest `.deb` package from the [release page](https://github.com/sharkdp/fd/releases) and install it via:
``` bash
sudo dpkg -i fd_6.1.0_amd64.deb  # adapt version number and architecture
```

### On Arch Linux

You can install [the fd-rs package](https://www.archlinux.org/packages/community/x86_64/fd-rs/) from the official repos:
```
pacman -S fd-rs
```

### On Void Linux

You can install `fd` via xbps-install:
```
xbps-install -S fd
```

### On macOS

You can install [this Homebrew package](http://braumeister.org/formula/fd):
```
brew install fd
```

### On Windows

You can download pre-built binaries from the [release page](https://github.com/sharkdp/fd/releases).

### On NixOS / via Nix

You can use the [Nix package manager](https://nixos.org/nix/) to install `fd`:
```
nix-env -i fd
```

### On FreeBSD

You can install `sysutils/fd` via portmaster:
```
portmaster sysutils/fd
```

### From source

With Rust's package manager [cargo](https://github.com/rust-lang/cargo), you can install *fd* via:
```
cargo install fd-find
```
Note that rust version *1.20.0* or later is required.

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
    -I, --no-ignore         Do not respect .(git)ignore files
        --no-ignore-vcs     Do not respect .gitignore files
    -s, --case-sensitive    Case-sensitive search (default: smart case)
    -i, --ignore-case       Case-insensitive search (default: smart case)
    -a, --absolute-path     Show absolute instead of relative paths
    -L, --follow            Follow symbolic links
    -p, --full-path         Search full path (default: file-/dirname only)
    -0, --print0            Separate results by the null character
    -h, --help              Prints help information
    -V, --version           Prints version information

OPTIONS:
    -d, --max-depth <depth>       Set maximum search depth (default: none)
    -t, --type <filetype>...      Filter by type: f(ile), d(irectory), (sym)l(ink)
    -e, --extension <ext>...      Filter by file extension
    -x, --exec <cmd>...           Execute a command for each search result
    -E, --exclude <pattern>...    Exclude entries that match the given glob pattern
    -c, --color <when>            When to use colors: never, *auto*, always
    -j, --threads <num>           Set number of threads to use for searching & executing

ARGS:
    <pattern>    the search pattern, a regular expression (optional)
    <path>...    the root directory for the filesystem search (optional)
```

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

If we want so search a specific directory, it can be given as a second argument to *fd*:
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
this behavior, we can use the `-I` (or `--ignore`) option:
``` bash
> fd num_cpu
> fd -I num_cpu
target/debug/deps/libnum_cpus-f5ce7ef99006aa05.rlib
```

To really search *all* files and directories, simply combine the hidden and ignore features to show
everything (`-HI`).

### Using fd with `xargs` or `parallel`

If we want to run a command on all search results, we can pipe the output to `xargs`:
``` bash
> fd -0 -e rs | xargs -0 wc -l
```
Here, the `-0` option tells *fd* to separate search results by the NULL character (instead of     .
newlines) In the same way, the `-0` option of `xargs` tells it to read the input in this way      .

### Using fd with `fzf`

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

# fd
[![Build Status](https://travis-ci.org/sharkdp/fd.svg?branch=master)](https://travis-ci.org/sharkdp/fd)
[![Version info](https://img.shields.io/crates/v/fd-find.svg)](https://crates.io/crates/fd-find)

*fd* is a simple, fast and user-friendly alternative to
[*find*](https://www.gnu.org/software/findutils/).

While it does not seek to mirror all of *find*'s powerful functionality, it provides sensible
(opinionated) defaults for [80%](https://en.wikipedia.org/wiki/Pareto_principle) of the use cases.

## Features
* Convenient syntax: `fd PATTERN` instead of `find -iname '*PATTERN*'`.
* Colorized terminal output (similar to *ls*).
* It's *fast* (see benchmarks below).
* Smart case: the search is case-insensitive by default. It switches to
  case-sensitive if the pattern contains an uppercase
  character[\*](http://vimdoc.sourceforge.net/htmldoc/options.html#'smartcase').
* Ignores hidden directories and files, by default.
* Ignores patterns from your `.gitignore`, by default.
* Regular expressions.
* Unicode-awareness.
* The command name is *50%* shorter[\*](https://github.com/ggreer/the_silver_searcher) than
  `find` :-).

## Demo

![Demo](http://i.imgur.com/kTMFSVU.gif)

## Colorized output
`fd` can colorize files by extension, just like `ls`. In order for this to work, the environment
variable [`LS_COLORS`](https://linux.die.net/man/5/dir_colors) has to be set. Typically, the value
of this variable is set by the `dircolors` command which provides a convenient configuration format
to define colors for different file formats.
On most distributions, `LS_COLORS` should be set already. If you are looking for alternative, more
complete (and more colorful) variants, see
[here](https://github.com/seebi/dircolors-solarized) or
[here](https://github.com/trapd00r/LS_COLORS).

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

## Install
With Rust's package manager [cargo](https://github.com/rust-lang/cargo), you can install *fd* via:
```
cargo install fd-find
```
Note that rust version *1.16.0* or later is required.
The release page of this repository also includes precompiled binaries for Linux.

On **macOS**, you can use [Homebrew](http://braumeister.org/formula/fd):
```
brew install fd
```

On **Arch Linux**, you can install the AUR package [fd-rs](https://aur.archlinux.org/packages/fd-rs/) via yaourt, or manually:
```
git clone https://aur.archlinux.org/fd-rs.git
cd fd-rs
makepkg -si
```

## Development
```bash
git clone https://github.com/sharkdp/fd

# Build
cd fd
cargo build

# Run unit tests
cargo test

# Run integration tests
cd tests
bash test.sh

# Install
cargo install
```

## Command-line options
```
USAGE:
    fd [FLAGS/OPTIONS] [<pattern>] [<path>]

FLAGS:
    -s, --case-sensitive    Case-sensitive search (default: smart case)
    -p, --full-path         Search full path (default: file-/dirname only)
    -H, --hidden            Search hidden files and directories
    -I, --no-ignore         Do not respect .(git)ignore files
    -L, --follow            Follow symlinks
    -0, --print0            Separate results by the null character
    -a, --absolute-path     Show absolute instead of relative paths
    -n, --no-color          Do not colorize output
    -h, --help              Prints help information
    -V, --version           Prints version information

OPTIONS:
    -d, --max-depth <depth>    Set maximum search depth (default: none)
    -j, --threads <threads>    The number of threads used for searching
    -t, --type <file-type>     The type of file to search for [values: f, file,
                               d, directory, s, symlink]

ARGS:
    <pattern>    the search pattern, a regular expression (optional)
    <path>       the root directory for the filesystem search (optional)
```

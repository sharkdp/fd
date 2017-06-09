# fd
[![Build Status](https://travis-ci.org/sharkdp/fd.svg?branch=master)](https://travis-ci.org/sharkdp/fd)

*fd* is a simple, fast and user-friendly alternative to
[*find*](https://www.gnu.org/software/findutils/).

While it does not seek to mirror all of *find*'s powerful functionality, it provides sensible
(opinionated) defaults for [80%](https://en.wikipedia.org/wiki/Pareto_principle) of the use cases.

## Features
* Convenient syntax: `fd PATTERN` instead of `find -iname '*PATTERN*'`.
* Smart case: the search is case-insensitive by default. It switches to
  case-sensitive if the pattern contains an uppercase
  character[\*](http://vimdoc.sourceforge.net/htmldoc/options.html#'smartcase').
* Colorized terminal output (similar to *ls*).
* Ignores hidden directories and files, by default.
* Ignores patterns from your `.gitignore`, by default.
* Regular expressions.
* Unicode-awareness.
* The command name is *50%* shorter[\*](https://github.com/ggreer/the_silver_searcher) than
  `find` :-).

## Demo

![Demo](http://i.imgur.com/iU6qkQj.gif)

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
A search in my home folder with ~150.000 subdirectories and ~1M files. The given options for
`fd` are needed for a fair comparison (otherwise `fd` is even faster by a factor of 5 because it
does not have to search hidden and ignored paths):
```
benchmarking bench/fd --hidden --no-ignore --full-path '.*[0-9]\.jpg$' ~
time                 2.800 s    (2.722 s .. 2.895 s)
                     1.000 R²   (1.000 R² .. 1.000 R²)
mean                 2.821 s    (2.810 s .. 2.831 s)
std dev              16.52 ms   (0.0 s .. 17.02 ms)
variance introduced by outliers: 19% (moderately inflated)

benchmarking bench/find ~ -iregex '.*[0-9]\.jpg$'
time                 5.593 s    (5.412 s .. 5.798 s)
                     1.000 R²   (0.999 R² .. 1.000 R²)
mean                 5.542 s    (5.502 s .. 5.567 s)
std dev              37.32 ms   (0.0 s .. 42.77 ms)
variance introduced by outliers: 19% (moderately inflated)
```
(benchmarking tool: [bench](https://github.com/Gabriel439/bench))

Both tools found the exact same 14030 files. Note that we have used the `-iregex` option for `find`
in order for both tools to perform a regular expression search. Both tools are comparably fast if
`-iname '*[0-9].jpg'` is used for `find`.

## Install
With [cargo](https://github.com/rust-lang/cargo), you can clone, build and install *fd* with a single command:
```
cargo install --git https://github.com/sharkdp/fd
```
Note that rust version *1.16.0* or later is required.
The release page of this repository also includes precompiled binaries for Linux.

## Development
```bash
cargo build --release
```

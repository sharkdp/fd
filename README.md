# fd
[![Build Status](https://travis-ci.org/sharkdp/fd.svg?branch=master)](https://travis-ci.org/sharkdp/fd)

*fd* is a simple, fast and user-friendly alternative to [*find*](https://www.gnu.org/software/findutils/).

While it does not seek to mirror all of *find*'s powerful functionality, it provides sensible (opinionated)
defaults for [80%](https://en.wikipedia.org/wiki/Pareto_principle) of the use cases.

## Features
* Convenient syntax: `fd PATTERN` instead of `find -iname '*PATTERN*'`.
* Smart case: the search is case-insensitive by default. It switches to
  case-sensitive if the pattern contains an uppercase
  character[\*](http://vimdoc.sourceforge.net/htmldoc/options.html#'smartcase').
* Ignores hidden directories and files by default.
* Colorized terminal output (similar to *ls*).
* Regular expressions by default.
* Unicode-aware.
* The command name is *50%* shorter[\*](https://github.com/ggreer/the_silver_searcher) than `find` :-).

## Demo

<a href="https://asciinema.org/a/120318" target="_blank"><img src="https://asciinema.org/a/120318.png" width="600" align="center" /></a>

## Build
```bash
cargo build --release
```

## Install
```
cargo install
```
The release page also includes precompiled binaries for Linux.

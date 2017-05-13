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

## Benchmark
A search in my home folder with ~80.000 subdirectories
and ~350.000 files. The `--hidden` for `fd` is needed
for a fair comparison, as *find* does this by default:
``` bash
> time fd --hidden '\.jpg$' > /dev/null
0,39s user 0,40s system 99% cpu 0,790 total

> time find -iname '*.jpg' > /dev/null 
0,36s user 0,42s system 98% cpu 0,789 total
```
Both tools found the exact same 5504 files and have
a comparable performance (averaged over multiple runs),
even though *fd* performs a regex search.
If we do the same for *find*, it is significantly slower:
``` bash
> time find -iregex '.*\.jpg$' > /dev/null
1,29s user 0,41s system 99% cpu 1,705 total
```


## Build
```bash
cargo build --release
```

## Install
```
cargo install
```
The release page also includes precompiled binaries for Linux.

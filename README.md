# fd
[![Build Status](https://travis-ci.org/sharkdp/fd.svg?branch=master)](https://travis-ci.org/sharkdp/fd)

`fd` is a modern, convenient and fast replacement for `find`.

## Features
* Convenient syntax: `fd PATTERN` instead of `find -iname '*PATTERN*'`.
* Colorized output.
* Regular expressions.
* Smart case: the search is case-insensitive by default, but will be
  case-sensitive if the pattern contains an uppercase character.
* Ignore hidden directories / files by default.
* Unicode-aware.
* The command name is *50%* shorter than `find` :-).

## Examples
<a href="https://asciinema.org/a/120318" target="_blank"><img src="https://asciinema.org/a/120318.png" width="600" /></a>

## Build
```bash
cargo build --release
```

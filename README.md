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
* The command name is *50%* shorter than `find` :-).

## Examples
``` bash
> fd
README.md
src
src/main.rs
Cargo.toml
LICENSE
Cargo.lock

> fd rs
src/main.rs

> fd '^[A-Z]+$'
LICENSE
```

## Build
```bash
cargo build --release
```

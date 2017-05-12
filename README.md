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

## Demo

<a href="https://asciinema.org/a/120318" target="_blank"><img src="https://asciinema-bb-eu.s3.amazonaws.com/uploads/png/120318/288aa6ca16c8863a6a723dd2cb9fbba17be6bb0c.png?Signature=PrEm7H2tlLQIDc5I17MB7siRyUU%3D&AWSAccessKeyId=AKIAI2DOCAQ34YNJM3GA&Expires=1494713189" width="600" align="center" /></a>

## Build
```bash
cargo build --release
```

## Install
```
cargo install
```
The release page also includes precompiled binaries for Linux.

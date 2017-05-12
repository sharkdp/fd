# fd
A modern, convenient and fast replacement for `find`.

**Features:**
* Easy syntax: `fd PATTERN` instead of `find -iname '*PATTERN*'`.
* Colored output.
* Regular expressions.
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
cargo build
```

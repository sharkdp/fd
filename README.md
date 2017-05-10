# fnd
A modern, convenient and fast replacement for `find`.

**Features:**
* Easy syntax. `fnd PATTERN` instead of `find -iname '*PATTERN*'`.
* Colored output.
* Regular expressions.
* The command name is *25%* shorter than `find` :-).

## Examples
``` bash
> fnd
src
src/fnd.cpp
README.md
LICENSE
CMakeLists.txt

> fnd cpp
src/fnd.cpp

> fnd '[A-Z].*'
README.md
LICENSE
CMakeLists.txt
```

## Dependencies
* g++ `>=4.9`
* boost `>=1.60`

## Build
```bash
cmake .
make
```

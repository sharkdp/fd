# fnd
A modern, convenient and fast replacement for 'find'

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

## Build
```bash
cmake .
make
```

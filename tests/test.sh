#!/bin/bash

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

fd="${SCRIPT_DIR}/../target/debug/fd"

export reset='\x1b[0m'
export bold='\x1b[01m'
export green='\x1b[32;01m'
export red='\x1b[31;01m'

set -eou pipefail

suite() {
    echo
    echo -e "${bold}$1${reset}"
    echo
}

expect() {
    expected_output="$1"
    shift

    tmp_expected="$(mktemp)"
    tmp_output="$(mktemp)"

    echo "$expected_output" > "$tmp_expected"

    "$fd" "$@" | sort > "$tmp_output"

    echo -ne "  ${bold}▶${reset} Testing 'fd $*' ... "

    if diff -q "$tmp_expected" "$tmp_output" > /dev/null; then
        echo -e "${green}✓ okay${reset}"
    else
        echo -e "${red}❌FAILED${reset}"

        echo -ne "\nShowing diff between ${red}expected${reset} and "
        echo -e "${green}actual${reset} output:\n"

        diff -C3 --label expected --label actual --color \
            "$tmp_expected" "$tmp_output" || true
        # exit 1
    fi
}

root=$(mktemp --directory)

cd "$root"

# Setup test environment

mkdir -p one/two/three

touch a.cpp
touch one/b.cpp
touch one/two/c.cpp
touch one/two/C.cpp
touch one/two/three/d.cpp
touch ignored.cpp

touch .hidden.cpp

echo "ignored.cpp" > .gitignore

ln -s one/two symlink


# Run the tests

suite "Simple tests"
expect "a.cpp" a.cpp
expect "one/b.cpp" b.cpp
expect "one/two/three/d.cpp" d.cpp
expect "a.cpp
one/b.cpp
one/two/c.cpp
one/two/C.cpp
one/two/three/d.cpp" cpp


suite "Smart case"
expect "one/two/c.cpp
one/two/C.cpp" c.cpp
expect "one/two/C.cpp" C.cpp


suite "Case-sensitivity (--sensitive)"
expect "one/two/C.cpp" --sensitive C.cpp


suite "Hidden files (--hidden)"
expect "a.cpp
.hidden.cpp
one/b.cpp
one/two/c.cpp
one/two/C.cpp
one/two/three/d.cpp" --hidden cpp


suite "Ignored files (--no-ignore)"
expect "a.cpp
ignored.cpp
one/b.cpp
one/two/c.cpp
one/two/C.cpp
one/two/three/d.cpp" --no-ignore cpp

expect "a.cpp
.hidden.cpp
ignored.cpp
one/b.cpp
one/two/c.cpp
one/two/C.cpp
one/two/three/d.cpp" --hidden --no-ignore cpp


suite "Symlinks (--follow)"
expect "one/two/c.cpp
one/two/C.cpp
symlink/c.cpp
symlink/C.cpp" --follow c.cpp


suite "Maximum depth (--max-depth)"
expect "a.cpp
one/b.cpp
one/two/c.cpp
one/two/C.cpp" --max-depth 3
expect "a.cpp
one/b.cpp" --max-depth 2
expect "a.cpp" --max-depth 1

# All done
echo

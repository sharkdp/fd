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
        exit 1
    fi
}

root=$(mktemp --directory)

cd "$root"

# Setup test environment

mkdir -p one/two/three

touch a.foo
touch one/b.foo
touch one/two/c.foo
touch one/two/C.Foo
touch one/two/three/d.foo
mkdir one/two/three/directory_foo
touch ignored.foo
touch .hidden.foo
ln -s one/two symlink

echo "ignored.foo" > .ignore


# Run the tests

suite "Simple tests"
expect "a.foo" a.foo
expect "one/b.foo" b.foo
expect "one/two/three/d.foo" d.foo
expect "a.foo
one/b.foo
one/two/c.foo
one/two/C.Foo
one/two/three/d.foo
one/two/three/directory_foo" foo
expect "a.foo
one
one/b.foo
one/two
one/two/c.foo
one/two/C.Foo
one/two/three
one/two/three/d.foo
one/two/three/directory_foo
symlink" # run 'fd' without arguments

suite "Explicit root path"
expect "one/b.foo
one/two/c.foo
one/two/C.Foo
one/two/three/d.foo
one/two/three/directory_foo" foo one
expect "one/two/three/d.foo
one/two/three/directory_foo" foo one/two/three
(
cd one/two
expect "../../a.foo
../b.foo
c.foo
C.Foo
three/d.foo
three/directory_foo" foo ../../
)

suite "Regex searches"
expect "a.foo
one/b.foo
one/two/c.foo
one/two/C.Foo" '[a-c].foo'
expect "a.foo
one/b.foo
one/two/c.foo" --case-sensitive '[a-c].foo'



suite "Smart case"
expect "one/two/c.foo
one/two/C.Foo" c.foo
expect "one/two/C.Foo" C.Foo
expect "one/two/C.Foo" Foo


suite "Case-sensitivity (--case-sensitive)"
expect "one/two/c.foo" --case-sensitive c.foo
expect "one/two/C.Foo" --case-sensitive C.Foo


suite "Full path search (--full-path)"
expect "one/two/three/d.foo
one/two/three/directory_foo" --full-path 'three.*foo'
expect "a.foo" --full-path '^a\.foo$'


suite "Hidden files (--hidden)"
expect "a.foo
.hidden.foo
one/b.foo
one/two/c.foo
one/two/C.Foo
one/two/three/d.foo
one/two/three/directory_foo" --hidden foo


suite "Ignored files (--no-ignore)"
expect "a.foo
ignored.foo
one/b.foo
one/two/c.foo
one/two/C.Foo
one/two/three/d.foo
one/two/three/directory_foo" --no-ignore foo

expect "a.foo
.hidden.foo
ignored.foo
one/b.foo
one/two/c.foo
one/two/C.Foo
one/two/three/d.foo
one/two/three/directory_foo" --hidden --no-ignore foo


suite "Symlinks (--follow)"
expect "one/two/c.foo
one/two/C.Foo
symlink/c.foo
symlink/C.Foo" --follow c.foo


suite "Maximum depth (--max-depth)"
expect "a.foo
one
one/b.foo
one/two
one/two/c.foo
one/two/C.Foo
one/two/three
symlink" --max-depth 3
expect "a.foo
one
one/b.foo
one/two
symlink" --max-depth 2
expect "a.foo
one
symlink" --max-depth 1


suite "Absolute paths (--absolute-path)"
expect "$root/a.foo
$root/one/b.foo
$root/one/two/c.foo
$root/one/two/C.Foo
$root/one/two/three/d.foo
$root/one/two/three/directory_foo" --absolute-path foo


suite "Invalid UTF-8"
touch "$(printf 'test-invalid-utf8-\xc3.txt')"
expect "$(printf 'test-invalid-utf8-\ufffd.txt')" test-invalid-utf8

# All done
echo

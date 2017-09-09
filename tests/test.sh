#!/bin/bash

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

fd="${SCRIPT_DIR}/../target/debug/fd"

MKTEMP_TEMPLATE="fd-tests.XXXXXXXXXX"

# Stabilize sort
export LC_ALL="C"
export LC_CTYPE="UTF-8"

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

    tmp_expected="$(mktemp -t "$MKTEMP_TEMPLATE")"
    tmp_output="$(mktemp -t "$MKTEMP_TEMPLATE")"

    echo "$expected_output" > "$tmp_expected"

    "$fd" "$@" | sed -e 's/\x0/NULL\n/g' | sort -f > "$tmp_output"

    echo -ne "  ${bold}▶${reset} Testing 'fd $*' ... "

    if diff -q "$tmp_expected" "$tmp_output" > /dev/null; then
        echo -e "${green}✓ okay${reset}"

        rm -f "$tmp_expected" "$tmp_output"
    else
        echo -e "${red}❌FAILED${reset}"

        echo -ne "\nShowing diff between ${red}expected${reset} and "
        echo -e "${green}actual${reset} output:\n"

        diff -C3 --label expected --label actual \
            "$tmp_expected" "$tmp_output" || true

        rm -f "$tmp_expected" "$tmp_output"

        exit 1
    fi
}

root=$(mktemp -d -t "$MKTEMP_TEMPLATE")

cd "$root"

# Setup test environment

mkdir -p one/two/three

touch a.foo
touch one/b.foo
touch one/two/c.foo
touch one/two/C.Foo2
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
one/two/C.Foo2
one/two/three/d.foo
one/two/three/directory_foo" foo
expect "a.foo
one
one/b.foo
one/two
one/two/c.foo
one/two/C.Foo2
one/two/three
one/two/three/d.foo
one/two/three/directory_foo
symlink" # run 'fd' without arguments

suite "Explicit root path"
expect "one/b.foo
one/two/c.foo
one/two/C.Foo2
one/two/three/d.foo
one/two/three/directory_foo" foo one
expect "one/two/three/d.foo
one/two/three/directory_foo" foo one/two/three
(
cd one/two
expect "../../a.foo
../b.foo
c.foo
C.Foo2
three/d.foo
three/directory_foo" foo ../../
)

suite "Regex searches"
expect "a.foo
one/b.foo
one/two/c.foo
one/two/C.Foo2" '[a-c].foo'
expect "a.foo
one/b.foo
one/two/c.foo" --case-sensitive '[a-c].foo'



suite "Smart case"
expect "one/two/c.foo
one/two/C.Foo2" c.foo
expect "one/two/C.Foo2" C.Foo
expect "one/two/C.Foo2" Foo


suite "Case-sensitivity (--case-sensitive)"
expect "one/two/c.foo" --case-sensitive c.foo
expect "one/two/C.Foo2" --case-sensitive C.Foo


suite "Full path search (--full-path)"
expect "one/two/three/d.foo
one/two/three/directory_foo" --full-path 'three.*foo'
expect "a.foo" --full-path '^a\.foo$'


suite "Hidden files (--hidden)"
expect ".hidden.foo
a.foo
one/b.foo
one/two/c.foo
one/two/C.Foo2
one/two/three/d.foo
one/two/three/directory_foo" --hidden foo


suite "Ignored files (--no-ignore)"
expect "a.foo
ignored.foo
one/b.foo
one/two/c.foo
one/two/C.Foo2
one/two/three/d.foo
one/two/three/directory_foo" --no-ignore foo

expect ".hidden.foo
a.foo
ignored.foo
one/b.foo
one/two/c.foo
one/two/C.Foo2
one/two/three/d.foo
one/two/three/directory_foo" --hidden --no-ignore foo


suite "Symlinks (--follow)"
expect "one/two/c.foo
one/two/C.Foo2
symlink/c.foo
symlink/C.Foo2" --follow c.foo

suite "Null separator (--print0)"
expect "a.fooNULL
one/b.fooNULL
one/two/C.Foo2NULL
one/two/c.fooNULL
one/two/three/d.fooNULL
one/two/three/directory_fooNULL" --print0 foo


suite "Maximum depth (--max-depth)"
expect "a.foo
one
one/b.foo
one/two
one/two/c.foo
one/two/C.Foo2
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

abs_path=$(python -c "import os; print(os.path.realpath('$root'))")

suite "Absolute paths (--absolute-path)"
expect "$abs_path/a.foo
$abs_path/one/b.foo
$abs_path/one/two/c.foo
$abs_path/one/two/C.Foo2
$abs_path/one/two/three/d.foo
$abs_path/one/two/three/directory_foo" --absolute-path foo
expect "$abs_path/a.foo
$abs_path/one/b.foo
$abs_path/one/two/c.foo
$abs_path/one/two/C.Foo2
$abs_path/one/two/three/d.foo
$abs_path/one/two/three/directory_foo" foo "$abs_path"

# All done
echo

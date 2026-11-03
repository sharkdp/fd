#!/bin/sh
# Run from the repository root with Rust and Cargo on PATH.
set -eu

check_features() {
    actual=$(sh scripts/build-features.sh "$1")
    if [ "$actual" != "$2" ]; then
        echo "$1: expected '$2', got '$actual'" >&2
        exit 1
    fi
}

for target in x86_64-unknown-linux-gnu x86_64-unknown-linux-musl \
    aarch64-unknown-linux-gnu aarch64-unknown-linux-musl \
    arm-unknown-linux-gnueabihf i686-unknown-linux-gnu; do
    check_features "$target" use-jemalloc
done
for target in i686-unknown-linux-musl arm-unknown-linux-musleabihf \
    riscv64gc-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin \
    x86_64-pc-windows-msvc i686-pc-windows-msvc aarch64-pc-windows-msvc \
    x86_64-pc-windows-gnu aarch64-pc-windows-gnullvm aarch64-linux-android \
    x86_64-unknown-freebsd x86_64-unknown-openbsd x86_64-unknown-illumos; do
    check_features "$target" ''
done

actual=$(CARGO_BUILD_TARGET=i686-unknown-linux-musl sh scripts/build-features.sh)
test -z "$actual"
actual=$(CARGO_BUILD_TARGET=i686-unknown-linux-musl sh scripts/build-features.sh x86_64-unknown-linux-gnu)
test "$actual" = use-jemalloc
if sh scripts/build-features.sh invalid-target-for-fd-test 2>/dev/null; then
    echo 'Invalid targets must fail feature selection' >&2
    exit 1
fi

# These targets used to silently discard an explicit allocator request.
for target in riscv64gc-unknown-linux-gnu i686-unknown-linux-musl; do
    tree=$(cargo tree --locked --target "$target" --features use-jemalloc)
    echo "$tree" | grep -q tikv-jemallocator
    tree=$(cargo tree --locked --target "$target")
    if echo "$tree" | grep -q tikv-jemallocator; then
        echo "$target: jemalloc must be opt-in" >&2
        exit 1
    fi
done
echo 'Build feature checks passed'

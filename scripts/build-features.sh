#!/usr/bin/env bash
# Select release features without preventing explicit allocator opt-ins.
# With no argument, use Cargo's environment target override or the rustc host.
set -eu

target=${1:-${CARGO_BUILD_TARGET:-}}
if [ -n "$target" ]; then
    cfg=$(rustc --print cfg --target "$target")
else
    cfg=$(rustc --print cfg)
fi

# Keep the allocator choices used by existing releases. macOS is excluded
# because of the Catalina issue: https://github.com/sharkdp/fd/issues/498.
case "$cfg" in
    *'target_os="windows"'* | *'target_os="android"'* | \
    *'target_os="macos"'* | *'target_os="freebsd"'* | \
    *'target_os="openbsd"'* | *'target_os="illumos"'* | \
    *'target_arch="riscv64"'*) exit 0 ;;
    *'target_env="musl"'*)
        case "$cfg" in
            *'target_pointer_width="32"'*) exit 0 ;;
        esac ;;
esac
echo use-jemalloc

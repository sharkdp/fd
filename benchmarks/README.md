# Allocator Benchmark Suite for fd

This directory contains benchmarks for comparing jemalloc vs mimalloc performance in the fd file finder utility.

## Overview

The benchmark suite builds two versions of fd—one linked with jemalloc and one with mimalloc—and runs comparative benchmarks using [hyperfine](https://github.com/sharkdp/hyperfine).

## Prerequisites

Install hyperfine:
```bash
cargo install hyperfine
```

## Quick Start

Run the default benchmark:
```bash
./benchmarks/allocator-comparison.sh
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `--rebuild` | Force rebuild of both versions | Only build if missing |
| `--runs N` | Number of hyperfine runs | 20 |
| `--warmup N` | Number of warmup runs | 3 |
| `--search-path` | Path to search | `/home/nanw/References/fd` |

## Example Commands

### Run with more iterations:
```bash
./benchmarks/allocator-comparison.sh --runs 50 --warmup 5
```

### Force rebuild and use different search path:
```bash
./benchmarks/allocator-comparison.sh --rebuild --search-path /home
```

## Build Commands

If you prefer to build manually:

### Build with jemalloc:
```bash
cargo build --profile=release-jemalloc --features=use-jemalloc
```

### Build with mimalloc:
```bash
cargo build --profile=release-mimalloc --features=use-mimalloc
```

### Run hyperfine manually:
```bash
hyperfine \
    --runs 20 \
    --warmup 3 \
    'target/release-jemalloc/fd --type f .' \
    'target/release-mimalloc/fd --type f .'
```

## Cargo Profiles

The following custom profiles are defined in `Cargo.toml`:

- `release-jemalloc`: Release build with jemalloc enabled
- `release-mimalloc`: Release build with mimalloc enabled

Both inherit from the `release` profile with LTO, strip, and single codegen unit.

## Features

- `use-jemalloc`: Links with tikv-jemallocator (legacy, use mimalloc for better performance)
- `use-mimalloc`: Links with mimalloc-sys (default on supported platforms)

## Benchmark Tests

The suite runs the following test cases:

1. **Recursive file listing**: `fd --type f .`
2. **Pattern matching**: `fd --type f '\.rs$'`
3. **Hidden files**: `fd --hidden .`
4. **Case-insensitive search**: `fd --ignore-case --type f 'readme'`
5. **Extension search**: `fd --type f --extension md`

## Interpreting Results

The benchmark output shows:
- Mean execution time for each allocator
- Standard deviation
- Speed difference (which allocator is faster)
- Statistical significance based on the number of runs

Typical factors that may affect results:
- File system type and cache state
- Number of files in the search path
- Pattern complexity
- System load during benchmark

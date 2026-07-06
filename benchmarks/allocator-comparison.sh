#!/bin/bash
#
# Benchmark Script: jemalloc vs mimalloc for fd
# 
# This script builds fd with both allocators and runs hyperfine benchmarks
# to compare their performance.
#
# Usage: ./benchmarks/allocator-comparison.sh [OPTIONS]
#
# Options:
#   --rebuild      Force rebuild of both versions (default: only build if binaries don't exist)
#   --runs N       Number of hyperfine runs (default: 20)
#   --warmup N     Number of warmup runs (default: 3)
#   --search-path  Path to search (default: /home/nanw/References/fd)
#

set -e

# Default values
REBUILD=false
NUM_RUNS=20
NUM_WARMUP=3
SEARCH_PATH="/home/nanw/References/fd"
BIN_DIR="target/release-jemalloc"
MIMALLOC_BIN_DIR="target/release-mimalloc"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --rebuild)
            REBUILD=true
            shift
            ;;
        --runs)
            NUM_RUNS="$2"
            shift 2
            ;;
        --warmup)
            NUM_WARMUP="$2"
            shift 2
            ;;
        --search-path)
            SEARCH_PATH="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --rebuild      Force rebuild of both versions"
            echo "  --runs N       Number of hyperfine runs (default: 20)"
            echo "  --warmup N     Number of warmup runs (default: 3)"
            echo "  --search-path  Path to search (default: /home/nanw/References/fd)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "========================================"
echo "  fd Allocator Benchmark: jemalloc vs mimalloc"
echo "========================================"
echo ""
echo "Configuration:"
echo "  Search path: $SEARCH_PATH"
echo "  Number of runs: $NUM_RUNS"
echo "  Warmup runs: $NUM_WARMUP"
echo ""

# Check if hyperfine is installed
if ! command -v hyperfine &> /dev/null; then
    echo "Error: hyperfine is not installed."
    echo "Install it with: cargo install hyperfine"
    exit 1
fi

# Build jemalloc version
JEMALLOC_BIN="$BIN_DIR/fd"
echo ">>> Building fd with jemalloc..."
if [ ! -f "$JEMALLOC_BIN" ] || [ "$REBUILD" = true ]; then
    cargo build --profile=release-jemalloc --features=use-jemalloc
else
    echo "    Using existing binary: $JEMALLOC_BIN"
fi

# Build mimalloc version
MIMALLOC_BIN="$MIMALLOC_BIN_DIR/fd"
echo ">>> Building fd with mimalloc..."
if [ ! -f "$MIMALLOC_BIN" ] || [ "$REBUILD" = true ]; then
    cargo build --profile=release-mimalloc --features=use-mimalloc
else
    echo "    Using existing binary: $MIMALLOC_BIN"
fi

echo ""
echo ">>> Verifying binaries..."
if [ ! -f "$JEMALLOC_BIN" ]; then
    echo "Error: jemalloc binary not found at $JEMALLOC_BIN"
    exit 1
fi
if [ ! -f "$MIMALLOC_BIN" ]; then
    echo "Error: mimalloc binary not found at $MIMALLOC_BIN"
    exit 1
fi

# Verify allocator linkage
echo ""
echo ">>> Allocator verification:"
JEMALLOC_CHECK=$(ldd "$JEMALLOC_BIN" 2>/dev/null | grep -i jemalloc || echo "not found")
MIMALLOC_CHECK=$(ldd "$MIMALLOC_BIN" 2>/dev/null | grep -i mimalloc || echo "not found")
echo "    jemalloc binary: $JEMALLOC_CHECK"
echo "    mimalloc binary: $MIMALLOC_CHECK"

echo ""
echo "========================================"
echo "  Running Benchmarks"
echo "========================================"
echo ""

# Benchmark commands
# Use the fd binary to search the repository itself as a representative workload
echo ">>> Benchmark 1: Recursive file listing (all files)"
echo "    Command: fd --type f . $SEARCH_PATH"
echo ""

hyperfine \
    --runs "$NUM_RUNS" \
    --warmup "$NUM_WARMUP" \
    --name "fd-jemalloc" \
    --prepare "" \
    "$JEMALLOC_BIN --type f . $SEARCH_PATH" \
    --name "fd-mimalloc" \
    "$MIMALLOC_BIN --type f . $SEARCH_PATH"

echo ""
echo ">>> Benchmark 2: Search with pattern matching (*.rs files)"
echo "    Command: fd --type f '\.rs$' $SEARCH_PATH"
echo ""

hyperfine \
    --runs "$NUM_RUNS" \
    --warmup "$NUM_WARMUP" \
    --name "fd-jemalloc" \
    "$JEMALLOC_BIN --type f '\.rs$' $SEARCH_PATH" \
    --name "fd-mimalloc" \
    "$MIMALLOC_BIN --type f '\.rs$' $SEARCH_PATH"

echo ""
echo ">>> Benchmark 3: Hidden files search"
echo "    Command: fd --hidden . $SEARCH_PATH"
echo ""

hyperfine \
    --runs "$NUM_RUNS" \
    --warmup "$NUM_WARMUP" \
    --name "fd-jemalloc" \
    "$JEMALLOC_BIN --hidden . $SEARCH_PATH" \
    --name "fd-mimalloc" \
    "$MIMALLOC_BIN --hidden . $SEARCH_PATH"

echo ""
echo ">>> Benchmark 4: Case-insensitive search"
echo "    Command: fd --ignore-case --type f 'readme' $SEARCH_PATH"
echo ""

hyperfine \
    --runs "$NUM_RUNS" \
    --warmup "$NUM_WARMUP" \
    --name "fd-jemalloc" \
    "$JEMALLOC_BIN --ignore-case --type f 'readme' $SEARCH_PATH" \
    --name "fd-mimalloc" \
    "$MIMALLOC_BIN --ignore-case --type f 'readme' $SEARCH_PATH"

echo ""
echo ">>> Benchmark 5: Extension-based search"
echo "    Command: fd --type f --extension md $SEARCH_PATH"
echo ""

hyperfine \
    --runs "$NUM_RUNS" \
    --warmup "$NUM_WARMUP" \
    --name "fd-jemalloc" \
    "$JEMALLOC_BIN --type f --extension md $SEARCH_PATH" \
    --name "fd-mimalloc" \
    "$MIMALLOC_BIN --type f --extension md $SEARCH_PATH"

echo ""
echo "========================================"
echo "  Benchmark Complete"
echo "========================================"
echo ""
echo "Build artifacts:"
echo "  jemalloc: $JEMALLOC_BIN"
echo "  mimalloc: $MIMALLOC_BIN"
echo ""
echo "To run additional comparisons manually:"
echo "  hyperfine \"$JEMALLOC_BIN [args]\" \"$MIMALLOC_BIN [args]\""

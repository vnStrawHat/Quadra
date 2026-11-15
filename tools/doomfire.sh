#!/usr/bin/env bash
# tools/doomfire.sh [seconds]: 1Term's terminal throughput (specs/14, "Terminal throughput").
# Builds the fire demo and 1Term's bench example in release, prints the engine's ceiling
# (doomfire-feed, no console, no window), then runs the demo in a space of a real 1Term
# window through the pseudo console and prints what the demo measured.
set -euo pipefail
cd "$(dirname "$0")/.."
seconds="${1:-15}"
tools/cargo.sh build --release -p quadra-bench-doomfire 2>&1 | grep -E "^(error|warning)" -A12 || true
tools/cargo.sh build --release -p oneterm --example doomfire 2>&1 | grep -E "^(error|warning)" -A12 || true
mkdir -p .tmp/1term-bench
target/release/doomfire-feed.exe --seconds 5 --size 200x50
out="$(pwd)/.tmp/1term-bench/doomfire.txt"
DOOMFIRE="$(pwd)/target/release/doomfire.exe" DOOMFIRE_OUT="$out" DOOMFIRE_SECONDS="$seconds" \
	target/release/examples/doomfire.exe
echo "1term: $(cat "$out")"

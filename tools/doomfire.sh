#!/usr/bin/env bash
# tools/doomfire.sh [seconds]: OneTerm's terminal throughput (specs/14, "Terminal throughput").
# Builds the fire demo and OneTerm's bench example in the dist profile, prints the engine's ceiling
# (doomfire-feed, no console, no window), then runs the demo in a space of a real OneTerm
# window through the pseudo console and prints what the demo measured.
set -euo pipefail
cd "$(dirname "$0")/.."
seconds="${1:-15}"
tools/cargo.sh build --profile dist -p quadra-bench-doomfire 2>&1 | grep -E "^(error|warning)" -A12 || true
tools/cargo.sh build --profile dist -p oneterm --example doomfire 2>&1 | grep -E "^(error|warning)" -A12 || true
mkdir -p .tmp/oneterm-bench
rm -rf .tmp/oneterm-bench/defaults # a window of the default settings, whatever a run before changed
tools/conpty.sh > /dev/null
target/dist/doomfire-feed.exe --seconds 5 --size 200x50
target/dist/doomfire-conpty.exe --seconds 8 --size 147x40
out="$(pwd)/.tmp/oneterm-bench/doomfire.txt"
DOOMFIRE="$(pwd)/target/dist/doomfire.exe" DOOMFIRE_OUT="$out" DOOMFIRE_SECONDS="$seconds" \
	target/dist/examples/doomfire.exe
echo "oneterm: $(cat "$out")"

#!/usr/bin/env bash
# tools/bench.sh [scene...]: builds the bench app (target/release/compare.exe). With scene
# names (list, grid, document, editor, editor-scroll) it runs each without a window, as fast as it goes, and prints
# what a frame did and how long each pass took (docs/benchmarks.md).
set -euo pipefail
cd "$(dirname "$0")/.."
tools/cargo.sh build --release -p quadra-bench-compare 2>&1 | grep -E "^(error|warning)" -A12 || true
for scene in "$@"; do
	echo "== $scene"
	BENCH_HEADLESS=1 BENCH_PHASES=1 BENCH_FRAMES="${BENCH_FRAMES:-3000}" target/release/compare.exe "$scene" | grep phases
done

#!/usr/bin/env bash
# tools/bench.sh [scene...]: builds the bench app (target/dist/compare.exe). With scene
# names (list, grid, document, editor, editor-scroll) it runs each without a window, as fast as it goes, and prints
# what a frame did and how long each pass took (docs/benchmarks.md).
#
# tools/bench.sh compare [runs]: builds the app and the runner (target/dist/run.exe) and
# measures the app's real windows from outside, `runs` rounds (3 by default), into
# .tmp/bench/compare.json (docs/benchmarks.md, "Method").
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "${1:-}" = compare ]; then
	# One at a time: built together, the app would take the runner's features of the OS crate.
	tools/cargo.sh build --profile dist -p quadra-bench-compare
	tools/cargo.sh build --profile dist -p quadra-bench-runner
	mkdir -p .tmp/bench
	exec target/dist/run.exe -runs "${2:-3}" -out .tmp/bench/compare.json quadra=target/dist/compare.exe
fi
tools/cargo.sh build --profile dist -p quadra-bench-compare 2>&1 | grep -E "^(error|warning)" -A12 || true
for scene in "$@"; do
	echo "== $scene"
	BENCH_HEADLESS=1 BENCH_PHASES=1 BENCH_FRAMES="${BENCH_FRAMES:-3000}" target/dist/compare.exe "$scene" | grep phases
done

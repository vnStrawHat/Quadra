#!/usr/bin/env bash
# tools/bench.sh [scene...]: builds the bench app (target/dist/compare.exe). With scene
# names (list, grid, document, editor, editor-scroll) it runs each without a window, as fast as it goes, and prints
# what a frame did and how long each pass took (docs/benchmarks.md).
#
# tools/bench.sh compare [runs]: builds the app and the runner (target/dist/run.exe) and
# measures the app's real windows from outside, `runs` rounds (3 by default), into
# .tmp/bench/compare.json (docs/benchmarks.md, "Method").
#
# tools/bench.sh incremental: what a change costs against the size of the UI (specs/14, "What a
# change costs"): the cells scene at 600, 2,400 and 9,600 numbers, changing one, all, or
# rebuilding the grid each frame, without a window, after 1,200 frames that let the middle
# number reach its longest.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "${1:-}" = incremental ]; then
	tools/cargo.sh build --profile dist -p quadra-bench-compare 2>&1 | grep -E "^(error|warning)" -A12 || true
	printf '%-6s %-8s %9s %8s %10s %9s %9s\n' cells change "laid out" painted instances uploaded "us/frame"
	for n in 600 2400 9600; do
		for change in one all rebuild; do
			line=$(BENCH_CELLS=$n BENCH_CHANGE=$change BENCH_HEADLESS=1 BENCH_PHASES=1 BENCH_WARMUP="${BENCH_WARMUP:-1200}" BENCH_FRAMES="${BENCH_FRAMES:-600}" \
				target/dist/compare.exe cells | grep phases)
			us=$(($(echo "$line" | sed -E 's/.*update ([0-9]+) layout ([0-9]+) paint ([0-9]+) compose ([0-9]+) draw ([0-9]+).*/\1+\2+\3+\4+\5/')))
			count() { echo "$line" | grep -oE "[0-9]+ $1" | cut -d' ' -f1; }
			printf '%-6s %-8s %9s %8s %10s %9s %9s\n' "$n" "$change" "$(count 'laid out')" "$(count painted)" "$(count 'instances written')" "$(count uploaded)" "$us"
		done
	done
	exit 0
fi
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

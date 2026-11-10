#!/usr/bin/env bash
# OneTerm under the fire load test (specs/12, Terminal throughput): builds fire and OneTerm with
# clang -prod, runs OneTerm with fire as the shell's program (no shell, no terminal logging) and
# prints each run's result line and the median fps.
#
#   bench/fire/run.sh [--host bundled|system] [--runs 3] [--seconds 15] [--size 160x45]
#                     [--no-build] [--exe oneterm.exe] [--stats]
#
# Everything lands in .tmp/fire-run/. Keep the window visible while it runs: a hidden or
# minimised window draws nothing. --stats passes OneTerm's --stats: its per-second counters
# land in data-N/stats.txt. --exe runs a OneTerm built elsewhere (an A/B of two builds).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
host=bundled runs=3 seconds=15 size=160x45 build=1 stats='' exe=''
while [ $# -gt 0 ]; do
	case "$1" in
	--host) host=$2; shift ;;
	--runs) runs=$2; shift ;;
	--seconds) seconds=$2; shift ;;
	--size) size=$2; shift ;;
	--no-build) build=0 ;;
	--stats) stats=--stats ;;
	--exe) exe=$2; shift ;;
	*) echo "fire/run.sh: unknown argument $1" >&2; exit 2 ;;
	esac
	shift
done
OUT="$ROOT/.tmp/fire-run"
DIR="$OUT/$host"
mkdir -p "$DIR"
if [ "$build" = 1 ]; then
	tools/v.sh -cc clang -prod -o "$OUT/fire.exe" bench/fire
	tools/v.sh -cc clang -prod -o "$OUT/oneterm.exe" apps/oneterm
fi
# Defender has deleted fresh -prod builds on the development machine: say so plainly.
one=${exe:-$OUT/oneterm.exe}
for f in "$OUT/fire.exe" "$one"; do
	[ -r "$f" ] || { echo "fire/run.sh: $f is missing (not built, or quarantined)" >&2; exit 1; }
done
cp -f "$one" "$DIR/oneterm.exe"
case "$host" in
bundled) tools/conpty.sh "$DIR" ;;
system) rm -rf "$DIR/conpty.dll" "$DIR/x64" ;;
*) echo "fire/run.sh: --host is bundled or system" >&2; exit 2 ;;
esac

fps=()
for n in $(seq 1 "$runs"); do
	data="$DIR/data-$n"
	rm -rf "$data"
	mkdir -p "$data"
	report="$(cd "$data" && pwd -W)/fire.txt"
	fire="$(cd "$OUT" && pwd -W)/fire.exe"
	cat >"$data/terminal.json" <<EOF
{
 "shell": {"kind": "custom", "program": "$fire",
  "args": ["--seconds", "$seconds", "--size", "$size", "--report", "$report"]},
 "logging": {"local": false}
}
EOF
	"$DIR/oneterm.exe" --data "$data" $stats 2>"$data/stderr.txt" &
	pid=$!
	for _ in $(seq 1 $(((${seconds%.*} + 30) * 10))); do
		[ -s "$data/fire.txt" ] && break
		sleep 0.1
	done
	sleep 0.5
	kill "$pid" 2>/dev/null || true
	wait "$pid" 2>/dev/null || true
	if [ ! -s "$data/fire.txt" ]; then
		echo "run $n: no result (see $data/stderr.txt)" >&2
		continue
	fi
	# OneTerm's stderr goes to its crash staging file once it starts: the --stats lines are there.
	cat "$data"/crashes/*.stderr* 2>/dev/null | grep '^stats:' >"$data/stats.txt" || true
	line=$(cat "$data/fire.txt")
	echo "run $n ($(head -n1 "$data/stderr.txt")): $line"
	read -r tw th <<<"$(sed -E 's/.*terminal ([0-9]+)x([0-9]+).*/\1 \2/' <<<"$line")"
	if [ "$tw" -lt "${size%x*}" ] || [ "$th" -lt "${size#*x}" ]; then
		echo "run $n: the terminal is smaller than $size: not counted" >&2
		continue
	fi
	fps+=("$(sed -E 's/.*: ([0-9.]+) fps.*/\1/' <<<"$line")")
done
[ ${#fps[@]} -gt 0 ] || exit 1
median=$(printf '%s\n' "${fps[@]}" | sort -n | sed -n "$(((${#fps[@]} + 1) / 2))p")
echo "fire/run.sh: $host host, median of ${#fps[@]}: $median fps"

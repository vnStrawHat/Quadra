#!/usr/bin/env bash
# OneTerm's memory in fixed scenarios (docs/benchmarks.md, OneTerm memory): starts OneTerm with
# --mem, lets each scenario run, reads the per-second `mem:` lines and prints the median of
# the runs for private working set, working set, commit and the collector's heap.
#
#   bench/mem/run.sh [--runs 3] [--no-build] [--exe oneterm.exe] [--out name] [scenario ...]
#
# Scenarios: idle (one cmd tab, 10 s), tabs5 (5 cmd tabs and the right dock, 10 s), fire (the
# fire load for 15 s, then 10 s idle), lines10k (`type` of 10 000 short lines, 10 s idle),
# full (the default 10 000-line history filled with full-width colored lines, 10 s idle).
# Builds with MSVC -prod: clang -prod builds of OneTerm are quarantined by Defender on the
# development machine. Keep the window visible: a hidden window draws nothing.
# Everything lands in .tmp/mem-run/<out>/ (default: base).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
runs=3 build=1 exe='' out=base scenarios=()
while [ $# -gt 0 ]; do
	case "$1" in
	--runs) runs=$2; shift ;;
	--no-build) build=0 ;;
	--exe) exe=$2; shift ;;
	--out) out=$2; shift ;;
	--*) echo "mem/run.sh: unknown argument $1" >&2; exit 2 ;;
	*) scenarios+=("$1") ;;
	esac
	shift
done
[ ${#scenarios[@]} -gt 0 ] || scenarios=(idle tabs5 fire lines10k full)
BASE="$ROOT/.tmp/mem-run"
DIR="$BASE/$out"
mkdir -p "$DIR"
if [ "$build" = 1 ]; then
	tools/v.sh -cc msvc -prod -o "$BASE/oneterm.exe" apps/oneterm
	[ -r "$BASE/fire.exe" ] || tools/v.sh -cc msvc -prod -o "$BASE/fire.exe" bench/fire
fi
one=${exe:-$BASE/oneterm.exe}
for f in "$BASE/fire.exe" "$one"; do
	[ -r "$f" ] || { echo "mem/run.sh: $f is missing (not built, or quarantined)" >&2; exit 1; }
done
cp -f "$one" "$DIR/oneterm.exe"
tools/conpty.sh "$DIR" >/dev/null
# Input files for the history scenarios, made once.
[ -s "$BASE/lines10k.txt" ] || python -I -c "
import sys
with open(sys.argv[1], 'w', newline='\r\n') as f:
    for i in range(10000):
        f.write('line %05d the quick brown fox jumps over the lazy dog 0123456789\n' % i)
" "$BASE/lines10k.txt"
[ -s "$BASE/full.txt" ] || python -I -c "
import sys
with open(sys.argv[1], 'w', newline='') as f:
    for i in range(12000):
        cells = ''.join('\x1b[38;5;%dm%s' % ((i + c) % 216 + 16, 'abcdefghij'[(i + c) % 10] * 8) for c in range(19))
        f.write(cells + '\x1b[0m\r\n')
" "$BASE/full.txt"
win() { (cd "$(dirname "$1")" && echo "$(pwd -W)/$(basename "$1")"); }

# config <data dir> <scenario>: terminal.json and ui_config.json for the scenario.
config() {
	local data=$1 sc=$2 shell='"kind": "cmd"'
	case "$sc" in
	fire) shell="\"kind\": \"custom\", \"program\": \"$(win "$BASE/fire.exe")\", \"args\": [\"--seconds\", \"15\", \"--size\", \"160x45\"]" ;;
	lines10k | full) shell="\"kind\": \"custom\", \"program\": \"cmd.exe\", \"args\": [\"/c\", \"type\", \"$(win "$BASE/$sc.txt" | tr / '\\\\')\"]" ;;
	esac
	shell=${shell//\\/\\\\}
	printf '{"shell": {%s}, "logging": {"local": false}}\n' "$shell" >"$data/terminal.json"
	if [ "$sc" = tabs5 ]; then
		echo '{"right_dock": {"local": true, "ssh": true}}' >"$data/ui_config.json"
	fi
}

for sc in "${scenarios[@]}"; do
	case "$sc" in
	idle | lines10k | full) secs=12 args=() ;;
	tabs5) secs=12 args=(--tabs 5) ;;
	fire) secs=27 args=() ;;
	*) echo "mem/run.sh: unknown scenario $sc" >&2; exit 2 ;;
	esac
	for n in $(seq 1 "$runs"); do
		data="$DIR/$sc-$n"
		rm -rf "$data"
		mkdir -p "$data"
		config "$data" "$sc"
		"$DIR/oneterm.exe" --data "$data" --mem "${args[@]}" 2>"$data/stderr.txt" &
		pid=$!
		sleep "$secs"
		kill "$pid" 2>/dev/null || true
		wait "$pid" 2>/dev/null || true
		# Once started, OneTerm's stderr goes to its crash staging file.
		cat "$data/stderr.txt" "$data"/crashes/*.stderr* 2>/dev/null | grep '^mem' >"$data/mem.txt" || true
		echo "$sc run $n: $(grep -c '^mem:' "$data/mem.txt") samples"
	done
done
python -I "$ROOT/bench/mem/summary.py" "$DIR" "${scenarios[@]}" | tee "$DIR/summary.md"

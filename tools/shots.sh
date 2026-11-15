#!/usr/bin/env bash
# Draws the pages of the gallery to pictures, without a window, and compares them with
# reference pictures when there are some:
#   tools/shots.sh [dark|light|contrast] [page ..]
# The pictures go to .tmp/shots/<theme>. QUADRA_REFERENCE names the folder that holds the
# reference pictures of that theme (the screenshots of the V gallery are the first ones).
# QUADRA_SCALE draws them as a display at that scale shows them (1.5: 150 %).
set -u
cd "$(dirname "$0")/.."
theme="${1:-dark}"
[ $# -gt 0 ] && shift
out=".tmp/shots/$theme"
tools/cargo.sh run -q -p quadra-bench-shots -- "$out" "$theme" "$@" || exit 1
if [ -n "${QUADRA_REFERENCE:-}" ]; then
	python -I tools/compare_shots.py "$out" "$QUADRA_REFERENCE"
fi

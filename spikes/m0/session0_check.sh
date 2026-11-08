#!/usr/bin/env bash
# M0 headless proof: run the WARP + DirectWrite renderer as SYSTEM in session 0
# (no desktop, no logged-in user) through a one-shot scheduled task.
# Needs an elevated Git Bash. Everything stays inside the project directory.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
net session >/dev/null 2>&1 || { echo "run this from an elevated (Administrator) Git Bash"; exit 2; }

mkdir -p .tmp/out
tools/v.sh -cc msvc -o .tmp/headless_png.exe spikes/m0/headless_png
EXE="$(cygpath -w "$ROOT/.tmp/headless_png.exe")"
OUT="$(cygpath -w "$ROOT/.tmp/out/session0.png")"
rm -f .tmp/out/session0.png .tmp/out/session0.png.log

export MSYS_NO_PATHCONV=1
schtasks /create /tn QuadraM0Headless /tr "$EXE $OUT 1.5" /sc once /st 23:59 /ru SYSTEM /rl HIGHEST /f >/dev/null
schtasks /run /tn QuadraM0Headless >/dev/null
for _ in $(seq 1 30); do
	[ -f .tmp/out/session0.png.log ] && break
	sleep 1
done
schtasks /delete /tn QuadraM0Headless /f >/dev/null

if [ ! -f .tmp/out/session0.png.log ]; then
	echo "FAIL no log written within 30 s"
	exit 1
fi
cat .tmp/out/session0.png.log
if grep -q "session 0" .tmp/out/session0.png.log && grep -q "^PASS" .tmp/out/session0.png.log; then
	cmp -s .tmp/out/session0.png .tmp/out/scene_150.png && echo "PASS session-0 PNG is byte-identical to the interactive render" \
		|| echo "NOTE session-0 PNG differs from .tmp/out/scene_150.png (inspect both)"
	echo "PASS headless render in session 0"
else
	echo "FAIL see log above"
	exit 1
fi

#!/usr/bin/env bash
# CI gate: formatting, portability rules, file size rule, headless tests.
#   tools/check.sh          everything
#   tools/check.sh quick    skip the tests
#   tools/check.sh prod     also run the tests as clang -prod builds (release codegen: the
#                           closure bug in docs/v-notes.md only shows in optimized builds)
set -euo pipefail
export PATH="/usr/bin:$PATH" # System32 first on PATH would pick up Windows find.exe
# export VJOBS="${VJOBS:-4}"   # 20 parallel V builds run the machine out of memory
cd "$(dirname "${BASH_SOURCE[0]}")/.."
fail=0
say() { printf '%s\n' "$*"; }

# 1. Formatting.
if ! tools/v.sh fmt -verify quadra apps tests examples bench >/dev/null 2>&1; then
	say 'check: files need `tools/v.sh fmt -w`:'
	tools/v.sh fmt -verify quadra apps tests examples bench 2>&1 | grep -v '^Already' | head -20
	fail=1
fi

# 2. Portability: OS code lives only in the backends. Everything else must build for any OS.
backends='^quadra/(platform/(win|headless)|gpu/d3d11|text/dwrite|native)/|^quadra/testing/pixels_'
os_files=$(find quadra apps -name '*_windows.c.v' -o -name '*_linux.c.v' -o -name '*_macos.c.v' -o -name '*_darwin.c.v' |
	grep -Ev "$backends" | grep -Ev '^quadra/platform/fs_' || true)
if [ -n "$os_files" ]; then
	say 'check: OS-specific files outside the backends:'
	say "$os_files"
	fail=1
fi
imports=$(grep -rnE '^import quadra\.(platform\.win|gpu\.d3d11|text\.dwrite)' quadra apps --include='*.v' |
	grep -Ev "$backends" | grep -Ev '_test\.v:|^apps/gallery/main\.v:|^apps/notes/main\.v:' || true)
if [ -n "$imports" ]; then
	say 'check: backend imports outside the backends (use quadra.native or quadra.platform):'
	say "$imports"
	fail=1
fi
cfns=$(grep -rnE '\bC\.[A-Z][A-Za-z0-9]+\(' quadra/core quadra/ui quadra/kit quadra/app quadra/layout quadra/text/*.v quadra/prim quadra/vector apps --include='*.v' || true)
if [ -n "$cfns" ]; then
	say 'check: Win32/C calls outside the backends:'
	say "$cfns"
	fail=1
fi
powershell=$(find . -path ./.tools -prune -o -path ./.cache -prune -o -name '*.ps1' -print || true)
if [ -n "$powershell" ]; then
	say 'check: PowerShell scripts are not allowed (bash + coreutils only):'
	say "$powershell"
	fail=1
fi

# 2b. Names the repo must never mention (the private list, .private/README.md; skipped
#     when the folder was not copied to this machine).
if [ -f .private/forbidden-names.txt ]; then
	named=$(git ls-files -co --exclude-standard 2>/dev/null | grep -v '^\.private/' |
		xargs -r grep -IlniwF -f .private/forbidden-names.txt 2>/dev/null || true)
	if [ -n "$named" ]; then
		say 'check: files naming a reference project (.private/forbidden-names.txt):'
		say "$named"
		fail=1
	fi
fi

# 3. No source file over 300 lines (generated tables excepted).
long=$(find quadra apps tests examples bench tools -name '*.v' -o -name '*.sh' -o -name '*.py' |
	grep -Ev '(icons|nfc_table)\.v$' | xargs wc -l | awk '$2 != "total" && $1 > 300 { print $2 " (" $1 " lines)" }')
if [ -n "$long" ]; then
	say 'check: files over 300 lines:'
	say "$long"
	fail=1
fi

# 3b. Closures in the framework (specs/14, avoiding closures): only the made-once ones are
#     left. The cap only goes down; a new `fn [..]` needs one of the four ways instead.
closure_cap=22
closures=$(find quadra -name '*.v' ! -name '*_test.v' -print0 | xargs -0 grep -hE '^[^/]*fn \[' | wc -l)
if [ "$closures" -gt "$closure_cap" ]; then
	say "check: $closures closures in quadra/ (cap $closure_cap); see specs/14, avoiding closures:"
	find quadra -name '*.v' ! -name '*_test.v' -print0 | xargs -0 grep -nE '^[^/]*fn \[' | head -40
	fail=1
fi

# 4. Programs no test builds (bench, tools): type-check them so API changes reach them too.
for prog in bench bench/*/ bench/compare/run; do
	[ -f "${prog%/}/main.v" ] || continue
	if ! out=$(tools/v.sh -check "$prog" 2>&1); then
		say "check: $prog does not build:"
		say "$out" | grep -E 'error' | head -5
		fail=1
	fi
done

# 5. Headless tests (no window station, GPU or login session needed except pixel tests,
#    which use WARP).
if [ "${1:-}" != quick ]; then
	if ! tools/v.sh test quadra tests examples apps >.tmp/check-tests.txt 2>&1; then
		say 'check: tests failed:'
		grep -E 'FAIL|> assert|Left|Right' .tmp/check-tests.txt | head -30
		fail=1
	else
		grep 'Summary' .tmp/check-tests.txt
	fi
fi
if [ "${1:-}" = prod ]; then
	if ! tools/v.sh -cc clang -prod test quadra tests examples apps >.tmp/check-prod.txt 2>&1; then
		say 'check: tests failed as clang -prod builds:'
		grep -E 'FAIL|> assert|Left|Right|panic' .tmp/check-prod.txt | head -30
		fail=1
	else
		grep 'Summary' .tmp/check-prod.txt
	fi
fi

if [ $fail -ne 0 ]; then
	exit 1
fi
say 'check: ok'

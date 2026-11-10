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
#     left, each with a comment saying why it does not leak. The cap only goes down.
closure_cap=3
closures=$(find quadra -name '*.v' ! -name '*_test.v' -print0 | xargs -0 grep -hE '^[^/]*fn \[' | wc -l)
if [ "$closures" -gt "$closure_cap" ]; then
	say "check: $closures closures in quadra/ (cap $closure_cap); see specs/14, avoiding closures:"
	find quadra -name '*.v' ! -name '*_test.v' -print0 | xargs -0 grep -nE '^[^/]*fn \[' | head -40
	fail=1
fi

# 3c. No capturing closures in apps or examples at all (specs/14, no closures in apps): V frees
#     none made outside a render frame, and a leak is worse than any memory it saves. `fn [` at
#     any position on a line, comments excepted; tests may use them.
app_closures=$(find apps examples -name '*.v' ! -name '*_test.v' -print0 |
	xargs -0 grep -nE '(^|[^A-Za-z0-9_])fn[[:space:]]*\[' | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' || true)
if [ -n "$app_closures" ]; then
	say 'check: capturing closures in apps/examples (cap 0; use the _with variants, specs/14):'
	say "$app_closures"
	fail=1
fi

# 3d. `unsafe` outside the FFI code (specs/14, unsafe cap): every block is a place the compiler
#     no longer checks, so the counts only go down. Counted per occurrence in non-test .v files,
#     comments stripped; `unsafe { nil }` (a nil pointer field) and `unsafe { self_ }` (the
#     builder idiom) do not count. The FFI modules and every *.c.v file are excluded: calling C
#     is what they are for.
unsafe_cap_quadra=130
unsafe_cap_apps=77
unsafe_ffi='^(quadra/(platform/win|gpu/d3d11|text/dwrite|native)/|.*\.c\.v$)'
unsafe_count() {
	find "$1" -name '*.v' ! -name '*_test.v' | grep -Ev "$unsafe_ffi" | xargs sed -e 's://.*$::' \
		-e 's/unsafe { nil }//g' -e 's/unsafe { self_ }//g' | grep -oE '\bunsafe\b' | wc -l
}
for tree in quadra apps; do
	cap_var="unsafe_cap_$tree"
	n=$(unsafe_count "$tree")
	if [ "$n" -gt "${!cap_var}" ]; then
		say "check: $n uses of unsafe in $tree/ (cap ${!cap_var}); see specs/14, unsafe cap. Added since HEAD:"
		git diff -U0 HEAD -- "$tree" | grep -E '^\+[^+].*\bunsafe\b' | grep -vE 'unsafe \{ (nil|self_) \}' | head -20 || true
		fail=1
	fi
done

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

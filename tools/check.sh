#!/usr/bin/env bash
# The gate: tools/check.sh [prod]. Everything must pass before work is called done.
#   prod: also builds and tests in release.
set -uo pipefail
cd "$(dirname "$0")/.."
# Text is read as UTF-8 whatever shell runs the gate: a shell that is not a login one has
# no locale, and Git Bash's grep 3.0 aborts on `-i -F -f` without a UTF-8 one.
export LC_ALL=C.UTF-8
fail=0
say() { echo "$*"; fail=1; }
files() { find . \( -name target -o -name .git -o -name .tmp -o -name .cache -o -name .private \) -prune -o -type f "$@" -print; }

# 1. No PowerShell (CLAUDE.md, hard rule 2).
ps1=$(files -name '*.ps1')
[ -z "$ps1" ] || say "check: PowerShell scripts are not allowed: $ps1"

# 2. Checks of this machine, kept out of git (where present).
[ ! -f .private/check.sh ] || . .private/check.sh

# 3. No source file over 300 lines.
long=$(files \( -name '*.rs' -o -name '*.sh' -o -name '*.py' \) | xargs -r wc -l |
	awk '$2 != "total" && $1 > 300 { print $2 " (" $1 " lines)" }')
[ -z "$long" ] || say "check: files over 300 lines: $long"

# 4. unsafe only at the OS boundary (specs/01, R16): every other crate inherits the
#    workspace lints, which forbid it.
for toml in crates/*/Cargo.toml bench/*/Cargo.toml; do
	case "$toml" in crates/quadra-native-win/* | crates/quadra-shell/*) continue ;; esac
	grep -A1 '^\[lints\]' "$toml" | grep -q '^workspace = true' ||
		say "check: $toml must have [lints] workspace = true (specs/01, R16)"
done

# 5. No look in the headless crate components (specs/17, "The crate"): no colour, role, paint, radius,
#    border, shadow or cursor; sizes are passed in by the kit.
look=$(grep -rnE '\b(Token|Paint|Color)\b|\.(bg|radius|radii|border|border_color|elevation|cursor|outline)\(' \
	crates/quadra-headless/src || true)
[ -z "$look" ] || say "check: quadra-headless names a look (specs/17): $look"

# 6. Dependencies (specs/01, R15).
tools/cargo.sh metadata --no-deps --format-version 1 | PYTHONUTF8=1 python tools/check_deps.py || fail=1

# 7. Format, lints, tests.
tools/cargo.sh fmt --all --check || say 'check: run tools/cargo.sh fmt --all'
tools/cargo.sh clippy --workspace --all-targets --quiet -- -D warnings || say 'check: clippy failed'
tools/cargo.sh test --workspace --quiet || say 'check: tests failed'
# The API documentation (specs/00, R5): every link resolves, no doc comment is malformed. By
# the toolchain's own cargo, into target/docs: the fork hangs at the end of a `doc` that had
# work to do (2026-11-14). The limit of time stays, should that cargo hang too. Its rustc and
# rustdoc are named by path: through the toolchain's proxies, a crate that ships a
# rust-toolchain.toml would be built by another toolchain.
sysroot="$(rustc --print sysroot)" project_tmp="$(cygpath -w "$PWD/.tmp")"
TMP="$project_tmp" TEMP="$project_tmp" RUSTDOCFLAGS='-D warnings' CARGO_TARGET_DIR=target/docs 	RUSTC="$sysroot/bin/rustc.exe" RUSTDOC="$sysroot/bin/rustdoc.exe" 	timeout 600 "$sysroot/bin/cargo.exe" doc --workspace --no-deps --quiet ||
	say 'check: the API documentation has warnings, or did not finish in 10 minutes'

if [ "${1:-}" = prod ]; then
	tools/cargo.sh build --workspace --release --quiet || say 'check: release build failed'
	tools/cargo.sh test --workspace --release --quiet || say 'check: release tests failed'
fi

[ $fail -eq 0 ] && echo 'check: ok'
exit $fail

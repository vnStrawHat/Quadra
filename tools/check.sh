#!/usr/bin/env bash
# The gate: tools/check.sh [prod]. Everything must pass before work is called done.
#   prod: also builds and tests in release.
set -uo pipefail
cd "$(dirname "$0")/.."
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

# 5. Dependencies (specs/01, R15).
tools/cargo.sh metadata --no-deps --format-version 1 | PYTHONUTF8=1 python tools/check_deps.py || fail=1

# 6. Format, lints, tests.
tools/cargo.sh fmt --all --check || say 'check: run tools/cargo.sh fmt --all'
tools/cargo.sh clippy --workspace --all-targets --quiet -- -D warnings || say 'check: clippy failed'
tools/cargo.sh test --workspace --quiet || say 'check: tests failed'
# The API documentation (specs/00, R5): every link resolves, no doc comment is malformed. By
# the toolchain's own cargo, into target/docs: the fork hangs at the end of a `doc` that had
# work to do (2026-11-14). The limit of time stays, should that cargo hang too.
toolchain_cargo="$(rustc --print sysroot)/bin/cargo.exe" project_tmp="$(cygpath -w "$PWD/.tmp")"
TMP="$project_tmp" TEMP="$project_tmp" RUSTDOCFLAGS='-D warnings' CARGO_TARGET_DIR=target/docs 	timeout 600 "$toolchain_cargo" doc --workspace --no-deps --quiet ||
	say 'check: the API documentation has warnings, or did not finish in 10 minutes'

if [ "${1:-}" = prod ]; then
	tools/cargo.sh build --workspace --release --quiet || say 'check: release build failed'
	tools/cargo.sh test --workspace --release --quiet || say 'check: release tests failed'
fi

[ $fail -eq 0 ] && echo 'check: ok'
exit $fail

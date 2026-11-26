#!/usr/bin/env bash
# Runs cargo for this repository from any directory: tools/cargo.sh test -p quadra-prim
#
# Cargo here is the fork that shares build output between projects (specs/00, Toolchain): it
# keeps intermediates under the cargo home and puts final binaries in ./target. Its background
# self-update is off, so the build tool changes only when someone updates it on purpose.
# Temp files stay inside the project.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$root/.tmp"
export CARGO_FORK_AUTO_UPDATE=0
TMP="$(cygpath -w "$root/.tmp" 2>/dev/null || echo "$root/.tmp")"
export TMP TEMP="$TMP"
cd "$root"
exec cargo "$@"

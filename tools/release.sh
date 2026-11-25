#!/usr/bin/env bash
# tools/release.sh: OneTerm's release asset (apps/oneterm/specs/oneterm.md, "Updates"), made by the app's own
# script, apps/oneterm/scripts/build-release.sh, with this repository's cargo, so that temp
# files stay in the project: dist/oneterm-<version>-<target>.zip, its .sha256, and the folder
# zipped. It publishes nothing.
set -euo pipefail
cd "$(dirname "$0")/.."
CARGO="$PWD/tools/cargo.sh" exec apps/oneterm/scripts/build-release.sh

#!/usr/bin/env bash
# The workspace on macOS (specs/23): tools/check_mac.sh
#   On a Mac (the CI's runner): clippy and the tests, natively.
#   Elsewhere (the Windows machine): clippy for the target, which needs no Mac: no linker
#   and no SDK. It finds what does not compile there, not what does not work.
set -euo pipefail
cd "$(dirname "$0")/.."
target=aarch64-apple-darwin

# Not on macOS yet; the list is what is left of the port (specs/23, "What is left out"):
# the Windows native crate and the tools that draw or measure with it (M2), and the apps,
# whose `main` asks for the shell's services of the system (M3; OneTerm a pty too, M4).
not_yet="quadra-native-win quadra-bench-shots quadra-bench-markdown-shots quadra-bench-compare
	quadra-bench-runner quadra-bench-doomfire oneterm-shots oneterm-app markdown"
# Checked on a Mac only: it holds C (ring, under russh), which another system compiles for
# macOS only with a compiler and an SDK it has not got.
needs_c="oneterm-ssh"

skip() { for name in "$@"; do printf -- '--exclude %s ' "$name"; done; }

if [ "$(uname)" = Darwin ]; then
	# shellcheck disable=SC2046
	tools/cargo.sh clippy --workspace --all-targets $(skip $not_yet) -- -D warnings
	# shellcheck disable=SC2046
	tools/cargo.sh test --workspace $(skip $not_yet)
else
	# shellcheck disable=SC2046
	tools/cargo.sh clippy --workspace --all-targets --keep-going --target "$target" \
		$(skip $not_yet $needs_c) -- -D warnings
fi
echo "check_mac: ok"

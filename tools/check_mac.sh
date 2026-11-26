#!/usr/bin/env bash
# The workspace on macOS (specs/23): tools/check_mac.sh
#   On a Mac (the CI's runner): clippy and the tests, natively.
#   Elsewhere (the Windows machine): clippy for the target, which needs no Mac: no linker
#   and no SDK. It finds what does not compile there, not what does not work.
set -euo pipefail
cd "$(dirname "$0")/.."
target=aarch64-apple-darwin

# Not on macOS yet; the list is what is left of the port (specs/23, "What is left out"):
# the Windows native crate and the tools that measure or draw with it (M2, M5).
not_yet="quadra-native-win quadra-bench-markdown-shots quadra-bench-compare
	quadra-bench-runner quadra-bench-doomfire"
# Never here: OneTerm has moved to its own repository, which ports it (specs/23, Goals).
not_ours="oneterm-app oneterm-ssh oneterm-vault oneterm-completion oneterm-shots"

skip() { for name in "$@"; do printf -- '--exclude %s ' "$name"; done; }

if [ "$(uname)" = Darwin ]; then
	# shellcheck disable=SC2046
	tools/cargo.sh clippy --workspace --all-targets --keep-going $(skip $not_yet $not_ours) -- -D warnings
	# The native crate's tests on their own, between two marks: the job says what each did.
	failed=0
	echo "== quadra-native-mac"
	tools/cargo.sh test -p quadra-native-mac --no-fail-fast || failed=1
	echo "== the rest"
	# shellcheck disable=SC2046
	tools/cargo.sh test --workspace --no-fail-fast $(skip $not_yet $not_ours quadra-native-mac) || failed=1
	[ "$failed" = 0 ]
else
	# shellcheck disable=SC2046
	tools/cargo.sh clippy --workspace --all-targets --keep-going --target "$target" \
		$(skip $not_yet $not_ours) -- -D warnings
fi
echo "check_mac: ok"

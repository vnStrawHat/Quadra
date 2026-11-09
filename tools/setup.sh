#!/usr/bin/env bash
# Set up a fresh clone: the project-local V 0.5.2 in .tools/v and a portable clang in
# .tools/llvm-mingw (downloaded into .cache), then a quick check that a release compiler works.
# Run from Git Bash: tools/setup.sh
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
want='V 0.5.2 7647ce1'
if [ ! -x .tools/v/v.exe ]; then
	mkdir -p .cache/downloads .tools
	zip=.cache/downloads/v_windows-0.5.2.zip
	[ -f "$zip" ] || curl -fL -o "$zip" https://github.com/vlang/v/releases/download/0.5.2/v_windows.zip
	rm -rf .cache/downloads/v-unzip && mkdir -p .cache/downloads/v-unzip
	python -I -m zipfile -e "$zip" .cache/downloads/v-unzip # Python 3, not unzip (not in Git Bash)
	mv .cache/downloads/v-unzip/v .tools/v
	rm -rf .cache/downloads/v-unzip
fi
got=$(.tools/v/v.exe version)
[ "$got" = "$want" ] || { echo "setup: .tools/v is '$got', want '$want'" >&2; exit 1; }
if [ ! -x .tools/llvm-mingw/bin/clang.exe ]; then
	llvm=llvm-mingw-20261104-ucrt-x86_64
	mkdir -p .cache/downloads .tools
	zip=.cache/downloads/$llvm.zip
	[ -f "$zip" ] || curl -fL -o "$zip" "https://github.com/mstorsjo/llvm-mingw/releases/download/20261104/$llvm.zip"
	echo "552191b5754c49a5a20723276a6fc56541ea1922c4c50b7b669022333fc1b40f  $zip" | sha256sum -c --quiet -
	rm -rf .cache/downloads/llvm-unzip && mkdir -p .cache/downloads/llvm-unzip
	python -I -m zipfile -e "$zip" .cache/downloads/llvm-unzip
	mv ".cache/downloads/llvm-unzip/$llvm" .tools/llvm-mingw
	rm -rf .cache/downloads/llvm-unzip
fi
# tools/v.sh applies the toolchain patch (tools/patches/v-closure.diff) on its first run.
tools/v.sh -cc clang -o .tmp/setup-hello.exe examples/counter >/dev/null ||
	{ echo 'setup: the clang build failed (see the output above)' >&2; exit 1; }
[ -d .private ] || echo 'setup: .private/ is missing: copy it from the old machine (see CLAUDE.md)'
echo "setup: ok ($got). Next: tools/check.sh"

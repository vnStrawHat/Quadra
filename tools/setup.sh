#!/usr/bin/env bash
# Set up a fresh clone: the project-local V 0.5.2 in .tools/v (downloaded into .cache), then
# a quick check that the C toolchain works. Run from Git Bash: tools/setup.sh
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
want='V 0.5.2 7647ce1'
if [ ! -x .tools/v/v.exe ]; then
	mkdir -p .cache/downloads .tools
	zip=.cache/downloads/v_windows-0.5.2.zip
	[ -f "$zip" ] || curl -fL -o "$zip" https://github.com/vlang/v/releases/download/0.5.2/v_windows.zip
	rm -rf .cache/downloads/v-unzip && mkdir -p .cache/downloads/v-unzip
	unzip -q "$zip" -d .cache/downloads/v-unzip
	mv .cache/downloads/v-unzip/v .tools/v
	rm -rf .cache/downloads/v-unzip
fi
got=$(.tools/v/v.exe version)
[ "$got" = "$want" ] || { echo "setup: .tools/v is '$got', want '$want'" >&2; exit 1; }
# tools/v.sh applies the toolchain patch (tools/patches/v-closure.diff) on its first run.
tools/v.sh -cc msvc -o .tmp/setup-hello.exe examples/counter >/dev/null ||
	{ echo 'setup: MSVC build failed: install Visual Studio Build Tools (C++ workload + Windows SDK)' >&2; exit 1; }
[ -d .private ] || echo 'setup: .private/ is missing: copy it from the old machine (see CLAUDE.md)'
echo "setup: ok ($got). Next: tools/check.sh"

#!/usr/bin/env bash
# Stage the bundled ConPTY host beside an executable (specs/17, specs/22): conpty.dll and
# x64/OpenConsole.exe from the OneTerm clone that tools/setup.sh fetches, each checked against
# the SHA-256 in conpty-manifest.json, plus conpty-NOTICE.txt (MIT). Copies go through a
# temporary name and only when the bytes differ: a running OneTerm may have the files open, and
# a file in use is kept as it is.
#
#   tools/conpty.sh <dir>     e.g. tools/conpty.sh .tmp (next to .tmp/oneterm.exe)
#
# Never commit the result: .tmp/ and dist/ are git-ignored.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/.cache/downloads/oneterm/crates/app/assets"
DEST=${1:?usage: tools/conpty.sh <dir>}
MANIFEST="$SRC/conpty-manifest.json"
[ -f "$MANIFEST" ] || { echo "conpty: $MANIFEST is missing: run tools/setup.sh" >&2; exit 1; }

manifest() { # <file key> <field>
	python -I -c 'import json,sys; m=json.load(open(sys.argv[1])); print(m["files"][sys.argv[2]][sys.argv[3]] if sys.argv[2] else m[sys.argv[3]])' "$MANIFEST" "$1" "$2"
}

mkdir -p "$DEST/x64"
for f in conpty.dll x64/OpenConsole.exe; do
	want=$(manifest "$f" sha256)
	got=$(sha256sum "$SRC/$f" | cut -d' ' -f1)
	[ "$got" = "$want" ] || { echo "conpty: $SRC/$f has SHA-256 $got, the manifest says $want" >&2; exit 1; }
	if [ -f "$DEST/$f" ] && [ "$(sha256sum "$DEST/$f" | cut -d' ' -f1)" = "$want" ]; then
		continue
	fi
	# A destination in use (a running OneTerm) cannot be replaced: keep it, as the original does.
	if ! { cp "$SRC/$f" "$DEST/$f.tmp" && mv -f "$DEST/$f.tmp" "$DEST/$f"; } 2>/dev/null; then
		rm -f "$DEST/$f.tmp"
		[ -f "$DEST/$f" ] || { echo "conpty: could not copy $f to $DEST" >&2; exit 1; }
		echo "conpty: $DEST/$f is in use: kept the old copy" >&2
	fi
done

version=$(manifest '' version)
cat >"$DEST/conpty-NOTICE.txt" <<EOF
conpty.dll and x64/OpenConsole.exe are unmodified x64 binaries of Windows Terminal
(https://github.com/microsoft/terminal), from the NuGet package
Microsoft.Windows.Console.ConPTY $version:
$(manifest '' source)

SHA-256:
  conpty.dll          $(manifest conpty.dll sha256)
  x64/OpenConsole.exe $(manifest x64/OpenConsole.exe sha256)

MIT License

Copyright (c) Microsoft Corporation.

Permission is hereby granted, free of charge, to any person obtaining a copy of this
software and associated documentation files (the "Software"), to deal in the Software
without restriction, including without limitation the rights to use, copy, modify,
merge, publish, distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice shall be included in all copies
or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
EOF
echo "conpty: staged ConPTY $version in $DEST"

#!/usr/bin/env bash
# tools/release.sh: OneTerm's release asset (specs/19, "Updates"): the package of
# tools/package.sh with LICENSE, NOTICE and the notices of what the binary holds, in target/release-zip/oneterm-<version>-<target>/,
# and that folder zipped beside it as oneterm-<version>-<target>.zip, the name the updater
# asks a GitHub release for (specs/22). It prints the zip's SHA-256; it publishes nothing.
set -euo pipefail
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' apps/oneterm/Cargo.toml | head -1)
name="oneterm-$version-x86_64-pc-windows-msvc"
out=target/release-zip
rm -rf "$out"
mkdir -p "$out"
PACKAGE_OUT="$out/$name" tools/package.sh oneterm
cp LICENSE NOTICE "$out/$name/"
# What the binary holds of others: its crates, the kit's icons and faces, the console host.
python tools/notices.py oneterm "$out/$name/THIRD-PARTY-NOTICES.txt"
python -c "import shutil, sys; shutil.make_archive(sys.argv[1] + '/' + sys.argv[2], 'zip', sys.argv[1], sys.argv[2])" "$out" "$name"
echo "release: $out/$name.zip ($(du -k "$out/$name.zip" | cut -f1) KB)"
echo "sha256: $(sha256sum "$out/$name.zip" | cut -d' ' -f1)"

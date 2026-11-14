#!/usr/bin/env bash
# tools/package.sh <example>: the release binary of examples/<example> (the package
# quadra-<example>, whose binary is <example>) with tools/app.manifest embedded by the
# linker, put in target/package/<example>/ (specs/16, "Packaging"). Fonts and images are in
# the binary (specs/15, "Assets"), so the folder holds it alone.
# ponytail: no installer, version resource or icon of the executable; an MSIX package when
# an app ships.
set -euo pipefail
cd "$(dirname "$0")/.."
name="${1:?usage: tools/package.sh <example>}"
[ -d "examples/$name" ] || { echo "package: no examples/$name" >&2; exit 1; }
manifest=$(cygpath -w "$PWD/tools/app.manifest")
# Only the final crate is linked again with these arguments; its dependencies are built once.
tools/cargo.sh rustc --release --quiet -p "quadra-$name" --bin "$name" -- \
	-C link-arg=/MANIFEST:EMBED -C "link-arg=/MANIFESTINPUT:$manifest"
out="target/package/$name"
rm -rf "$out"
mkdir -p "$out"
cp "target/release/$name.exe" "$out/"
grep -q PerMonitorV2 "$out/$name.exe" || { echo "package: the manifest is not in $out/$name.exe" >&2; exit 1; }
echo "package: $out/$name.exe ($(du -k "$out/$name.exe" | cut -f1) KB)"

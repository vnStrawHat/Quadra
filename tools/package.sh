#!/usr/bin/env bash
# tools/package.sh <example|oneterm|markdown>: the shipped binary (the dist profile) of examples/<example> (the package
# quadra-<example>, whose binary is <example>), of 1Term (apps/oneterm, binary oneterm) or of
# Markdown (apps/markdown); an app's icon is checked to be in its binary,
# with tools/app.manifest embedded by the linker, put in target/package/<name>/ (or in the
# folder PACKAGE_OUT names; specs/16,
# "Packaging"). Fonts and images are in the binary (specs/15, "Assets"); 1Term's folder also
# holds its console host and the host's notice (specs/19, "The console host").
# ponytail: no installer or version resource; an MSIX package when an app ships.
set -euo pipefail
cd "$(dirname "$0")/.."
name="${1:?usage: tools/package.sh <example|oneterm|markdown>}"
case "$name" in
oneterm) crate=oneterm bin=oneterm ;;
markdown) crate=markdown bin=markdown ;;
*)
	[ -d "examples/$name" ] || { echo "package: no examples/$name" >&2; exit 1; }
	crate="quadra-$name" bin="$name"
	;;
esac
manifest=$(cygpath -w "$PWD/tools/app.manifest")
# Only the final crate is linked again with these arguments; its dependencies are built once.
tools/cargo.sh rustc --profile dist --quiet -p "$crate" --bin "$bin" -- \
	-C link-arg=/MANIFEST:EMBED -C "link-arg=/MANIFESTINPUT:$manifest"
out="${PACKAGE_OUT:-target/package/$name}"
rm -rf "$out"
mkdir -p "$out"
cp "target/dist/$bin.exe" "$out/"
grep -q PerMonitorV2 "$out/$bin.exe" || { echo "package: the manifest is not in $out/$bin.exe" >&2; exit 1; }
if [ "$name" = markdown ] || [ "$name" = oneterm ]; then
	python tools/exe_icon.py "$out/$bin.exe" > /dev/null
fi
if [ "$name" = oneterm ]; then
	tools/conpty.sh "$out" > /dev/null
	cp tools/notices/conpty.txt "$out/THIRD-PARTY-NOTICES.txt"
fi
echo "package: $out/$bin.exe ($(du -k "$out/$bin.exe" | cut -f1) KB)"

#!/usr/bin/env bash
# tools/conpty.sh [folder...]: Microsoft's console host for 1Term (specs/01, R15; specs/19,
# "The console host"). Fetches the pinned NuGet package into .cache/conpty once, checks its
# SHA-256, and copies conpty.dll and OpenConsole.exe into each folder given, or into
# target/debug, target/release and target/release/examples.
set -euo pipefail
cd "$(dirname "$0")/.."
version=1.25.260930003
sha256=02b07b349af66d801159bdf9e440d4a1ce78bb951f37fc8609731665afdae7ee
cache=.cache/conpty
package="$cache/conpty.$version.nupkg"
unpacked="$cache/$version"
if [ ! -f "$unpacked/runtimes/win-x64/native/conpty.dll" ]; then
	mkdir -p "$cache"
	if [ ! -f "$package" ]; then
		url="https://api.nuget.org/v3-flatcontainer/microsoft.windows.console.conpty/$version/microsoft.windows.console.conpty.$version.nupkg"
		curl -sSfL -o "$package.part" "$url"
		mv "$package.part" "$package"
	fi
	echo "$sha256 *$package" | sha256sum -c --quiet - || { echo "conpty: $package is not the pinned package" >&2; rm -f "$package"; exit 1; }
	rm -rf "$unpacked"
	unzip -q -o "$package" -d "$unpacked"
fi
folders=("$@")
[ ${#folders[@]} -gt 0 ] || folders=(target/debug target/release target/release/examples)
for folder in "${folders[@]}"; do
	mkdir -p "$folder"
	cp "$unpacked/runtimes/win-x64/native/conpty.dll" "$unpacked/build/native/runtimes/x64/OpenConsole.exe" "$folder/"
done
echo "conpty: $version in ${folders[*]}"

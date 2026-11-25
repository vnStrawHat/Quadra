#!/usr/bin/env bash
# scripts/export.sh <folder> [quadra]: OneTerm as a repository of its own, written into an
# empty folder (specs/19, "The repository"). Run from Quadra's repository, where the app is
# built until the user moves it: the folder gets the app's tracked files as they are, the
# root manifest (Cargo.root.toml as Cargo.toml), the specs the code refers to (specs/19 and
# 20), and a lock file with the versions Quadra's has.
#
# Quadra is named by its git address at the commit checked out here, which has to be on the
# remote for anyone else to build it. With a second argument, the path of a checkout of
# Quadra, it is named by that path instead: for trying the folder on this machine.
# It changes nothing here and publishes nothing.
set -euo pipefail
cd "$(dirname "$0")/.."
out="${1:?usage: scripts/export.sh <folder> [path of a checkout of Quadra]}"
top="$(git rev-parse --show-toplevel)"
[ -f Cargo.root.toml ] || { echo "export: no Cargo.root.toml: this is not Quadra's repository" >&2; exit 1; }
if [ -n "${2:-}" ]; then
	from="path = \"$(cygpath -m "$2" 2>/dev/null || echo "$2")/crates/\\1\""
else
	rev="$(git rev-parse HEAD)"
	[ -n "$(git branch -r --contains "$rev")" ] ||
		{ echo "export: $rev is on no remote branch: push it first, or name a checkout of Quadra to try with" >&2; exit 1; }
	from="git = \"https://github.com/vnStrawHat/Quadra\", rev = \"$rev\""
fi
mkdir -p "$out"
[ -z "$(ls -A "$out")" ] || { echo "export: $out is not empty" >&2; exit 1; }
git ls-files -z | xargs -0 cp --parents -t "$out"
rm "$out/Cargo.root.toml" "$out/scripts/export.sh"
sed "s|@\(quadra[a-z-]*\)@|$from|" Cargo.root.toml > "$out/Cargo.toml"
mkdir -p "$out/specs"
cp "$top/specs/19-oneterm.md" "$top/specs/20-complete.md" "$out/specs/"
# Quadra's lock file: cargo keeps its versions and drops what this workspace does not build.
cp "$top/Cargo.lock" "$out/"
(cd "$out" && cargo metadata --format-version 1 > /dev/null)
echo "export: $out ($(cd "$out" && find . -type f | wc -l) files), Quadra by ${from%% *}"

#!/usr/bin/env bash
# Compiles the HLSL of the Windows renderer to bytecode with the Windows SDK's fxc. The .cso
# files are committed and embedded in the binary, so a normal build never needs fxc and start-up
# compiles no shader (specs/07, "Start-up"). Run it after editing a .hlsl file.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
kits='/c/Program Files (x86)/Windows Kits/10/bin'
fxc="${FXC:-$(ls -d "$kits"/*/x64/fxc.exe 2>/dev/null | sort | tail -1)}"
[ -x "$fxc" ] || { echo 'build_shaders: fxc.exe not found; install the Windows SDK or set FXC'; exit 1; }
dir="$root/crates/quadra-native-win/shaders"
for src in "$dir"/*.hlsl; do
	name="$(basename "$src" .hlsl)"
	"$fxc" -nologo -O3 -T vs_5_0 -E vs_main -Fo "$(cygpath -w "$dir/${name}_vs.cso")" "$(cygpath -w "$src")"
	"$fxc" -nologo -O3 -T ps_5_0 -E ps_main -Fo "$(cygpath -w "$dir/${name}_ps.cso")" "$(cygpath -w "$src")"
done
ls -l "$dir"/*.cso

#!/usr/bin/env bash
# Compile every HLSL file under quadra/gpu/d3d11/shaders to .cso bytecode with the SDK's fxc.
# The .cso files are committed and embedded, so normal builds never need fxc.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FXC="${FXC:-/c/Program Files (x86)/Windows Kits/10/bin/10.0.22621.0/x64/fxc.exe}"
DIR="$ROOT/quadra/gpu/d3d11/shaders"
for src in "$DIR"/*.hlsl; do
	name="$(basename "$src" .hlsl)"
	w() { cygpath -w "$1"; }
	"$FXC" -nologo -O3 -T vs_5_0 -E vs_main -Fo "$(w "$DIR/${name}_vs.cso")" "$(w "$src")"
	"$FXC" -nologo -O3 -T ps_5_0 -E ps_main -Fo "$(w "$DIR/${name}_ps.cso")" "$(w "$src")"
done

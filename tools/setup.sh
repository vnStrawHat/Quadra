#!/usr/bin/env bash
# Checks what a fresh machine needs to build Quadra and says what is missing. It installs
# nothing: the Rust toolchain, the cargo fork and the Visual Studio Build Tools are installed
# for the whole machine by their own installers (specs/00, Toolchain).
set -uo pipefail
cd "$(dirname "$0")/.."
missing=0
need() { command -v "$1" >/dev/null 2>&1 || { echo "setup: missing $1: $2"; missing=1; }; }

need git 'Git for Windows (Git Bash with coreutils)'
need python 'Python 3'
need rustup 'https://rustup.rs'
need cargo 'installed with rustup'
[ $missing -eq 0 ] || exit 1

# rustup reads rust-toolchain.toml here and installs the pinned toolchain on first use.
rustc --version || exit 1
cargo --version | grep -q 'huacnlee/cargo' ||
	echo 'setup: cargo is the standard one. It works; the fork that shares build output is at https://huacnlee.github.io/cargo/'

# The msvc target needs the linker and the Windows SDK of the Visual Studio Build Tools.
vswhere='/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe'
if [ -x "$vswhere" ] && [ -n "$("$vswhere" -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)" ]; then
	echo 'setup: ok'
else
	echo 'setup: install the Visual Studio Build Tools with "Desktop development with C++" (MSVC and the Windows SDK)'
	exit 1
fi

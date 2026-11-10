#!/usr/bin/env bash
# Run the project-local V compiler with every cache and temp dir inside the project.
set -euo pipefail
# From cmd, `bash` is often WSL's (WindowsApps\bash.exe): its exports never reach Windows
# programs, so V would put its temp files and caches outside the project. Use Git Bash
# (from cmd: tools\v.cmd).
if grep -qi microsoft /proc/version 2>/dev/null; then
	echo 'tools/v.sh: this is WSL bash; use Git Bash, or from cmd: tools\v.cmd <args>' >&2
	exit 1
fi
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -W 2>/dev/null || pwd)"
export VMODULES="$ROOT/.cache/vmodules"
export VCACHE="$ROOT/.cache/vcache"
export VTMP="$ROOT/.tmp/v"
export TEMP="$ROOT/.tmp" TMP="$ROOT/.tmp" TMPDIR="$ROOT/.tmp"
# V uploads generated C to bugs.vlang.io when the C compiler fails: never send project code out.
export V_C_ERROR_BUG_REPORT_DISABLED=1
# Always build with the garbage collector. V 0.5.2 silently builds MSVC targets without one
# (every allocation leaks); quadra.core refuses to compile without it.
case " ${VFLAGS:-} " in *' -gc '*) ;; *) export VFLAGS="${VFLAGS:-} -gc boehm_full_opt" ;; esac
# V compiles its bundled collector without the config header that turns on returning free
# heap blocks to the OS (USE_MUNMAP, on in the collector's own builds): without it a heap grown
# by a burst stays committed for good (quadra/core/trim.v, docs/benchmarks.md).
case " ${VFLAGS:-} " in *' -DUSE_MUNMAP '*) ;; *) export VFLAGS="$VFLAGS -cflags -DUSE_MUNMAP" ;; esac
# V passes its 8 MiB thread stack size to CreateThread as the size to commit, so every task,
# PTY and SSH thread costs 8 MiB of commit. 0 gives a thread the executable's default: 32 MiB
# reserved, committed a page at a time as it grows (pthreads keep their default).
case " ${VFLAGS:-} " in *' -thread-stack-size '*) ;; *) export VFLAGS="$VFLAGS -thread-stack-size 0" ;; esac
# The portable clang from tools/setup.sh, only for -cc clang: its bin folder also holds gcc, ld
# and as, which would change the compiler V picks by default. V's C fails clang's pointer-type
# check, and casts between pointer types. One flag per -cflags: `v test` splits a longer value.
case " $* " in *' -cc clang '*)
	export PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/.tools/llvm-mingw/bin:$PATH"
	export VFLAGS="$VFLAGS -cflags -Wno-incompatible-pointer-types -cflags -fno-strict-aliasing"
	;;
esac
mkdir -p "$VMODULES" "$VCACHE" "$VTMP"
# The stock closure thunk clobbers xmm15, a callee-saved register on Win64: callers optimized by
# MSVC or clang lose floats across closure calls. Keep the local toolchain patched (idempotent).
closure="$ROOT/.tools/v/vlib/builtin/closure/closure.c.v"
grep -q 'quadra patch: lets bench/leak see closure leaks' "$closure" ||
	patch -s -d "$ROOT/.tools/v" -p0 <"$ROOT/tools/patches/v-closure.diff"
# MSVC drops intermediate `*.tmp.obj` files into the working directory; remove them after each run.
status=0
"$ROOT/.tools/v/v.exe" "$@" || status=$?
rm -f -- ./*.tmp.obj 2>/dev/null || true # a concurrent build may still hold one
exit $status

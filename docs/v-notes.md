# V Notes for Quadra Contributors

Workarounds for V 0.5.2 behaviours hit while building Quadra. Each one cost a debugging session.

## Memory and closures (read first)

| Symptom | Cause | Rule |
|---|---|---|
| Working set climbs without bound (GBs) in MSVC builds; `gc_heap_usage()` is all zeros | V 0.5.2 builds MSVC targets with no garbage collector unless `-gc` is passed | `tools/v.sh` sets `VFLAGS=-gc boehm_full_opt`; `core.new_app` refuses to compile without `gcboehm` |
| Random NaN/inf floats (garbage geometry, index-out-of-range `-2147483648`) only in MSVC `-prod`, roughly every other run | The stock Win64 closure thunk passes the context pointer in `xmm15`, which is callee-saved on Win64: optimized callers keep floats there across closure calls. clang is affected too (a function holding 14 floats across a closure call came back wrong every time on the stock thunk) | `tools/v.sh` keeps the toolchain patched (`tools/patches/v-closure.diff`: `xmm5`, volatile and never an argument register) |
| Closures are never freed, with everything they captured (a captured password stays in memory for good) | V frees only named local closures that never escape (passed, stored or returned) at scope exit (`closure_try_destroy`), and closures created inside a `closure.Lifetime` frame | No `fn [` in apps or examples at all (check.sh, cap 0) and 3 made-once ones in quadra; use the `_with(data, fn)` variants (specs/14, "No closures in apps"); count with `life.live_closures()` |
| A closure leak with no idea where it comes from | — | Outside -prod the dispatch detector warns ("a handler of X made 1 closure(s) ..."); `QUADRA_TRACE_CLOSURES=1` prints a backtrace at each closure made in a dispatch (patched `closure.trace`) |
| A dropped buffer of tens of MB (Argon2id's 64 MiB) stays committed for good, and `trim_now` frees nothing: `free_bytes` is 0 after a collection | Boehm is conservative and interior pointers are on: one stale word anywhere in a stack or a scanned block that points into a block that large keeps it alive. It showed with real backends (pixel harness, the app) and not with the null device | Keep very large temporary buffers off the collector's heap: `C.calloc`, wiped and `C.free`d when done (Windows returns blocks that large to the OS at once). `tools/patches/v-argon2-release.diff` does it for `crypto.argon2`. Check on real backends (apps/oneterm/vault_memory_test.v) |
| `trim_now` and USE_MUNMAP change nothing in a default build: the heap never shrinks | Without `-cc`, V builds with tcc, which links the prebuilt `thirdparty/tcc/lib/libgc.a` (no USE_MUNMAP); tcc cannot compile the bundled `gc.c` (`__ATOMIC_RELAXED`) | Measure memory with `-cc msvc` or `-cc clang`; a tcc dev build keeps every heap peak |
| A method value (`obj.method` as a function value) leaks like `fn [obj]` | V builds a closure for a bound method | Pass a plain function and the object as data |
| Every thread costs 8 MiB of commit (a tab with its two PTY threads 16 MiB) | V passes its thread stack size (8 MiB) to `CreateThread` as the commit size, not as a reservation | `tools/v.sh` passes `-thread-stack-size 0`: the executable's default, 32 MiB reserved, committed as used |
| The collector's heap never shrinks: a burst's heap stays committed and resident; `unmapped_bytes` is always 0 | V compiles its bundled bdwgc without the config header, so `USE_MUNMAP` is off | `tools/v.sh` adds `-cflags -DUSE_MUNMAP` (it is also in V's cache key, so the collector object is rebuilt); `core` unmaps after a burst (trim.v) |
| The process grows to GBs, then the collector aborts with "Too many heap sections" | Not fragmentation: something allocates without bound (a loop that never ends, appending rows) until the collector runs out of heap sections (1024 in the bundled one, many GB) | Find the loop; tests that feed untrusted input import `quadra.testing.fuzz`, which caps the heap at 512 MB (`GC_set_max_heap_size`) so they die at once |
| `timeout 60 tools/v.sh test x_test.v` returns, and the test executable keeps running | `timeout` ends `v`, not the test process it started | Run long or risky tests from a built executable under `timeout` |
| `import builtin.closure` vanishes and the code turns into `builtin.closure.Lifetime` (does not compile) | vfmt 0.5.2 bug in some files of a non-main module | Use the `quadra.core.life` wrapper module |

## Language and codegen

| Symptom | Cause | Rule |
|---|---|---|
| `cannot pass expression as mut` on `ui.div().p(4)` | mut methods cannot be called on an rvalue | Builders take `self_ &T` and do `mut d := unsafe { self_ }` |
| `undeclared Holder_T_A_T_B` in generated C | V instantiates every 1-param generic method of a generic receiver with the type arguments of the others | Put generic methods with extra type params on the non-generic `Cx` where possible; build generic structs in free generic functions |
| C error `field 'any' has incomplete type` in `Listener_T_<Model>` | the spurious instantiation above (here `listener[E]` taking `observe[O]`'s model type) is emitted before its field types | Keep only one 1-param generic method on `ViewCx[T]` (`listener`); others are free functions |
| `generic parameter name needs to be exactly one char` | — | Use distinct single letters per method (`O`, `S`, `Q`, `E`) |
| `T.render undefined` in tests | the unused-code pass cannot see calls made through generic fn pointers in `_test.v` files | Mark test-file views `@[markused]`; `new_view` also makes a never-taken direct call |
| `method X.render is private` | generic code in another module calls it | View `render` methods must be `pub` |
| C error `identifier expected` | a variable named `near`/`far` (Windows macros) | Avoid `near`, `far`, `small`, `interface` as names |
| C error, or a segfault at run time, from `if c { a } else { b }` used as a call argument (seen with `unsafe { nil }` and inside `${...}`) | codegen bug | Assign the if-expression to a variable first |
| `for p in [opt_a, opt_b]` over options breaks codegen | codegen bug | Handle each option explicitly |
| `[]fn (){}` is split by `v fmt` | formatter bug | Use a named alias (`type Task = fn ()`) |
| map unusable after `m.move()` | `move()` leaves the source uninitialized | Reassign `m = map[K]V{}` after moving |
| `[]u64 == [alias_values]` fails in C | comparing arrays of an alias type | Cast elements to the base type |
| interface method calls fail on immutable receivers | methods declared after `mut:` need a mutable interface value | Put read-only methods before `mut:` in interfaces |
| Listener created later reads a garbage view id | a closure captured `mut cx` where cx is a `mut` parameter: that is a pointer into a stack frame that has returned | Capture `cx.entity` and use `core.listener_of[T, E](id, f)` in callbacks that outlive render (virtual list rows) |
| C error `call_bound_T_..._Theme` undeclared (or any generic function name with the type argument and no `___ptr__`) | taking a pointer to a generic function whose type argument is a pointer (`call_bound[D, &Theme, R]`): the reference is mangled `..._Theme*_...`, the instance `..._ptr__Theme_...` | Never pass a pointer type as a type argument there: take `&A` in the signature and pass `A` (`core.BoundRef`), or wrap the pointer in a struct |
| Wrong action type at run time (`typeof[A]().name` is "A", a key binding fires nothing) or C `cannot convert Edit to OpenPalette` | a generic function with the same name as a generic method in the module (`fn bind[D, A, R]` next to `App.bind[A]`): V mixes up their instances | Give generic functions names no generic method of the module uses (`core.bind_fn`) |
| Closures assigning to captured variables change nothing outside | closures capture by value | Compute values before the closure; pass results through models or heap structs |
| `unexpected token ,` on a call like `go(mut h, id)` | `go`, `dump`, `select`, `spawn` are keywords or builtins | Pick another name (`open_page`, `dump_node`, `picked`) |
| `main__worker_T_string undeclared` or `cannot convert string to int` in C | `spawn` of a generic function, or a closure inside a generic function | Type-erase: store the fn in a `Holder[R]` and pass a generic fn pointer (`run_work[R]`) to a non-generic function that spawns (core/tasks.v) |
| `generic struct init type parameter R must be within the parameters` | R only appears in a fn-type parameter | Build `S[R]{}` in a helper taking an `R` value; get a zero R from `[]R{len: 1}[0]` |
| A generic method encodes the wrong type (`json.encode` in `Settings[T].update`) | methods of a generic receiver are instantiated across all T | Store a per-T fn pointer at construction (`encode_value[T]`) and call it from the method |
| `base64.decode` panics on a byte above `z`, or reads out of bounds in -prod | vlib indexes its 123-entry table with every input byte unchecked; OSC 52, key files and vault envelopes reach it | `tools/patches/v-base64-bounds.diff` (applied by `tools/v.sh`): such input decodes to nothing |
| `x.json2` crashes on deeply nested input, loops forever growing memory on `[1` or `{"a":1` (a container cut off at the end), or panics while building an error message | no depth limit in its checker; EOF after a nested value re-checked the same value; the error context slice is unclamped | `tools/patches/v-json2-depth.diff` (applied by `tools/v.sh`): nesting past 256 is an error, so is EOF inside a container |
| `parse_uint('0x..', 16, ..)` returns an error | the prefix is not accepted with an explicit base | Strip `0x` |
| `string__plus` undeclared in C | a string `+` inside a multiple assignment (`a, b = s + t, x`) | Compute the string on its own line first |
| tcc crashes (no message) on an assignment | an if-expression nested inside another if-expression on the right side | Use an `if` statement, or a helper function |
| A word breaks in two ("Previe w") at exactly its measured width | float error in layout sums (`68.004 - 24 = 44.003998`) | `text.wrap` breaks with 0.01 px of slack (tests/shaping_test.v) |
| `unexpected token $` in a struct body | V has no `$if` among struct fields, and file suffixes (`_d_prod.v`, `_notd_prod.v`) do not follow `-prod` | Declare the struct twice under a top-level `$if prod { struct S {..} } $else { struct S {..} }` (prim/holder.v) |
| `missing return` with `$if prod { return a } $else { return b }` | the checker does not see a return in every comptime branch | Build a `mut` value, set the debug-only field inside `$if !prod`, return once |
| `int has no property` on `h.get[Gen[A, B]]().field` inside a generic function | a generic method with a generic type argument loses its return type there | Use a free generic function (`prim.held[T](h)`) and a local variable |
| `struct X was declared as private to module m` at a cast inside a generic function of another module | a cast expression `&T(p)` checks that T is visible where the cast is written | Return the `voidptr` as `&T` without a cast: it converts implicitly (prim.held) |
| `cannot cast string to voidptr` from a generic `voidptr(p)` with `p &T` | when T is `string`, V takes the cast for a string-to-pointer cast | `unsafe { voidptr(p) }` |
| notice `x is immutable, cannot have a mutable reference to an immutable object` on `mut v := prim.held[T](param)` | some generic instances treat the returned pointer as borrowed from the parameter | `mut v := unsafe { prim.held[T](param) }` (only quiets the notice) |
| A typed call inside a method of a generic struct gets another instance's type (a checked holder in `Settings[T].saved` was tagged with another settings type; `json.encode(*cx.peek(m))` in a per-T helper failed in C) | the generic-receiver mix-up above reaches every generic call in the method | Pass non-generic values out of the method (`App.model_holder(id, gen)`) and decode in a per-T function stored at construction |

## Found while building OneTerm

- An `if` expression passed as an argument in a chained call can build under tcc and crash at run time: assign it to a variable first.
- `json.encode` inside a `match` expression generates invalid C: encode first, then match.
- `cx.listener(named_fn)` can fail with an undeclared `call_listener_T_...`: pass an inline `fn`.
- A view's `render` in an app submodule needs `@[markused]`.
- `listener_with` with a pointer as its type argument breaks: wrap the pointer in a struct. The
  same holds for every `_with[D]` (spawn_task_with, update_with, kit.build_with, ...).
- `undeclared call_listener_T_...` in C when a module's listener is built only inside a function
  the unused-code pass keeps but never marks (kit/term in an app without terminals):
  `core.listener_of` makes a never-taken direct call to `call_listener` for this.
- A generic method with its own type parameter on a generic struct (`Settings[T].update_with[D]`)
  fails with `unknown type T` in the checker: make it a free function (`app.update_with`).
- Overlays opened from task or timer callbacks have a nil `cx.win`: open them from an event, or keep the window.
- `small` is a Windows macro once `net` is imported (hence `is_small` in the kit).
- `$embed_file` entries with the same base path collide: give the files distinct names.
- Under llvm-mingw `os.open_file` fails on `_fseeki64`, and `os.open_append` leaves the fd unset.

## Windows interop

- `tools/v.sh` sets `V_C_ERROR_BUG_REPORT_DISABLED=1`. Without it, V uploads the generated C (project code) to bugs.vlang.io whenever the C compiler fails.

- COM slots come from `tools/com_slots.sh`; never hand-count.
- Struct layouts of D3D11/DXGI are checked by `spikes/m0/layout_check` (MSVC only).
- tcc's import definitions lack newer kernel32/user32 functions: resolve them with `load_system_dll` + `dll_proc`.
- clang 23 rejects V's generated C with `incompatible pointer types`; V also casts between pointer types without `-fno-strict-aliasing`. `tools/v.sh` adds both flags for `-cc clang`.
- `v test -cflags "-a -b"` fails with `Unknown argument`: pass one flag per `-cflags`.
- Never test the real clipboard in automated runs: it overwrites the user's clipboard. Use the headless platform.

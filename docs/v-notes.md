# V Notes for Quadra Contributors

Workarounds for V 0.5.2 behaviours hit while building Quadra. Each one cost a debugging session.

## Memory and closures (read first)

| Symptom | Cause | Rule |
|---|---|---|
| Working set climbs without bound (GBs) in MSVC builds; `gc_heap_usage()` is all zeros | V 0.5.2 builds MSVC targets with no garbage collector unless `-gc` is passed | `tools/v.sh` sets `VFLAGS=-gc boehm_full_opt`; `core.new_app` refuses to compile without `gcboehm` |
| Random NaN/inf floats (garbage geometry, index-out-of-range `-2147483648`) only in MSVC `-prod`, roughly every other run | The stock Win64 closure thunk passes the context pointer in `xmm15`, which is callee-saved on Win64: optimized callers keep floats there across closure calls. clang is affected too (a function holding 14 floats across a closure call came back wrong every time on the stock thunk) | `tools/v.sh` keeps the toolchain patched (`tools/patches/v-closure.diff`: `xmm5`, volatile and never an argument register) |
| Closures are never freed, with everything they captured | V frees only named local closures at scope exit (`closure_try_destroy`) and closures created inside a `closure.Lifetime` frame | Quadra owns closure lifetimes (`quadra/core/tracked.v`, specs/14); keep `fn [..]` out of engine hot paths; count with `life.live_closures()` |
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
| `parse_uint('0x..', 16, ..)` returns an error | the prefix is not accepted with an explicit base | Strip `0x` |
| `string__plus` undeclared in C | a string `+` inside a multiple assignment (`a, b = s + t, x`) | Compute the string on its own line first |
| tcc crashes (no message) on an assignment | an if-expression nested inside another if-expression on the right side | Use an `if` statement, or a helper function |
| A word breaks in two ("Previe w") at exactly its measured width | float error in layout sums (`68.004 - 24 = 44.003998`) | `text.wrap` breaks with 0.01 px of slack (tests/shaping_test.v) |

## Windows interop

- `tools/v.sh` sets `V_C_ERROR_BUG_REPORT_DISABLED=1`. Without it, V uploads the generated C (project code) to bugs.vlang.io whenever the C compiler fails.

- COM slots come from `tools/com_slots.sh`; never hand-count.
- Struct layouts of D3D11/DXGI are checked by `spikes/m0/layout_check` (MSVC only).
- tcc's import definitions lack newer kernel32/user32 functions: resolve them with `load_system_dll` + `dll_proc`.
- clang 23 rejects V's generated C with `incompatible pointer types`; V also casts between pointer types without `-fno-strict-aliasing`. `tools/v.sh` adds both flags for `-cc clang`.
- `v test -cflags "-a -b"` fails with `Unknown argument`: pass one flag per `-cflags`.
- Never test the real clipboard in automated runs: it overwrites the user's clipboard. Use the headless platform.

// M0 spike: verifies the V language features the core API depends on.
// Each check prints one PASS/FAIL line.
module main

// --- 1. Type-erased listeners: typed handlers stored in one list, called from non-generic code.

struct EventCx[T] {
mut:
	notified bool
}

struct Click {
	x int
}

struct Key {
	code int
}

struct Counter {
mut:
	count int
}

struct Sidebar {
mut:
	label string
}

// Holder keeps the typed handler on the heap; Listener erases its type.
struct Holder[T, E] {
	f fn (mut v T, e &E, mut cx EventCx[T]) = unsafe { nil }
}

struct Listener {
	holder voidptr
	call   fn (holder voidptr, view voidptr, event voidptr) bool = unsafe { nil }
}

fn call_holder[T, E](holder voidptr, view voidptr, event voidptr) bool {
	unsafe {
		h := &Holder[T, E](holder)
		mut v := &T(view)
		e := &E(event)
		mut cx := EventCx[T]{}
		h.f(mut v, e, mut cx)
		return cx.notified
	}
}

fn make_listener[T, E](f fn (mut v T, e &E, mut cx EventCx[T])) Listener {
	h := &Holder[T, E]{
		f: f
	}
	return Listener{
		holder: voidptr(h)
		call:   call_holder[T, E]
	}
}

fn check_listeners() bool {
	mut c := &Counter{}
	mut s := &Sidebar{}
	mut list := []Listener{}
	list << make_listener[Counter, Click](fn (mut v Counter, e &Click, mut cx EventCx[Counter]) {
		v.count += e.x
		cx.notified = true
	})
	list << make_listener[Sidebar, Key](fn (mut v Sidebar, e &Key, mut cx EventCx[Sidebar]) {
		v.label = 'key ${e.code}'
	})
	click := Click{
		x: 5
	}
	key := Key{
		code: 42
	}
	n1 := list[0].call(list[0].holder, voidptr(c), voidptr(&click))
	n2 := list[1].call(list[1].holder, voidptr(s), voidptr(&key))
	return c.count == 5 && s.label == 'key 42' && n1 && !n2
}

// --- 2. Generic method with extra type params on a generic receiver.

struct Model[T] {
	id u32
}

struct ModelCx[T] {
mut:
	subs int
}

fn (mut cx ModelCx[T]) subscribe[M, E](_ Model[M], _ fn (mut v T, m Model[M], e &E, mut cx ModelCx[T])) {
	cx.subs++
}

fn check_generic_method() bool {
	mut cx := ModelCx[Sidebar]{}
	doc := Model[Counter]{
		id: 1
	}
	cx.subscribe[Counter, Click](doc, fn (mut v Sidebar, m Model[Counter], e &Click, mut cx ModelCx[Sidebar]) {
		v.label = 'x'
	})
	return cx.subs == 1
}

// --- 3. Generic interface Renderable[T] and type erasure into AnyView.

struct RenderCx[T] {
mut:
	frames int
}

interface Renderable[T] {
mut:
	render(mut cx RenderCx[T]) string
}

struct Hello {
mut:
	name string
}

fn (mut h Hello) render(mut cx RenderCx[Hello]) string {
	cx.frames++
	return 'hello ${h.name}'
}

struct AnyView {
	ptr    voidptr
	render fn (ptr voidptr) string = unsafe { nil }
}

fn render_erased[T](ptr voidptr) string {
	mut cx := RenderCx[T]{}
	mut v := unsafe { &T(ptr) }
	return v.render(mut cx)
}

fn to_any[T](v &T) AnyView {
	return AnyView{
		ptr:    voidptr(v)
		render: render_erased[T]
	}
}

fn use_renderable[T](mut r Renderable[T]) string {
	mut cx := RenderCx[T]{}
	return r.render(mut cx)
}

fn check_renderable() (bool, bool) {
	mut h := &Hello{
		name: 'quadra'
	}
	a := to_any(h)
	erased_ok := a.render(a.ptr) == 'hello quadra'
	mut r := Renderable[Hello](h)
	iface_ok := use_renderable[Hello](mut r) == 'hello quadra'
	return erased_ok, iface_ok
}

// --- 4. voidptr -> fn type cast, the basis of COM calls by vtable index.

type AddFn = fn (this voidptr, a int, b int) int

fn add_impl(_ voidptr, a int, b int) int {
	return a + b
}

struct FakeComObject {
	vtbl &voidptr
}

fn vtable_slot(obj voidptr, index int) voidptr {
	unsafe {
		vt := *(&&voidptr(obj))
		return vt[index]
	}
}

fn check_vtable_call() bool {
	mut table := [unsafe { nil }, unsafe { nil }, unsafe { nil }, voidptr(add_impl)]
	obj := &FakeComObject{
		vtbl: unsafe { &table[0] }
	}
	f := unsafe { AddFn(vtable_slot(voidptr(obj), 3)) }
	return f(voidptr(obj), 2, 3) == 5
}

// --- 5. malloc_noscan buffers (GC must not scan bulk plain data).

fn check_noscan() bool {
	n := 1 << 20
	buf := unsafe { &u8(malloc_noscan(n)) }
	unsafe {
		buf[0] = 7
		buf[n - 1] = 9
	}
	ok := unsafe { buf[0] == 7 && buf[n - 1] == 9 }
	unsafe { free(buf) }
	return ok
}

fn report(name string, ok bool) {
	println('${if ok { 'PASS' } else { 'FAIL' }} ${name}')
}

fn main() {
	report('type-erased listeners (Holder + generic trampoline)', check_listeners())
	report('generic method with extra type params on generic receiver', check_generic_method())
	erased, iface := check_renderable()
	report('AnyView erasure via generic fn pointer', erased)
	report('generic interface Renderable[T]', iface)
	report('voidptr to fn cast + vtable slot call', check_vtable_call())
	report('malloc_noscan under default GC', check_noscan())
}

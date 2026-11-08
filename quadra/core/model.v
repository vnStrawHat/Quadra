// Models: app-owned state with an ownership tree and automatic dependency tracking
// (specs/07-state.md). Handles are plain values; stale handles are detected.
module core

import quadra.core.life

pub struct Model[T] {
pub:
	id  u32
	gen u32
}

// View is a model whose value can render. Its type must provide
// `fn (mut v T) render(mut cx ViewCx[T]) Element`.
pub struct View[T] {
pub:
	id  u32
	gen u32
}

pub fn (v View[T]) model() Model[T] {
	return Model[T]{v.id, v.gen}
}

pub fn (v View[T]) any() AnyView {
	return AnyView{v.id, v.gen}
}

// AnyView is a type-erased view handle (window roots, child slots).
pub struct AnyView {
pub:
	id  u32
	gen u32
}

type RenderFn = fn (mut app App, win &Window, id u32) Element

struct Slot {
mut:
	gen       u32
	alive     bool
	ptr       voidptr
	owner     u32
	owned     []u32
	render    RenderFn = unsafe { nil }
	window    u32
	readers   map[u32]bool // views that read this model during their last render
	reads     []u32        // models this view read during its last render
	type_name string
	dirty     bool     // views: must render before their cached elements are reused
	cached    ?Element // views: the element tree from the last render
	lifetime  &life.Life = unsafe { nil } // views: closures of their renders (tracked.v)
	pinned    bool // the current render's closures may have escaped: never free them
}

struct Store {
mut:
	slots []Slot = [Slot{}] // id 0 is never used
	free  []u32
}

fn (mut s Store) alloc(ptr voidptr, owner u32, type_name string) (u32, u32) {
	mut id := u32(0)
	if s.free.len > 0 {
		id = s.free.pop()
	} else {
		s.slots << Slot{}
		id = u32(s.slots.len - 1)
	}
	mut slot := &s.slots[id]
	slot.gen++
	slot.alive = true
	slot.ptr = ptr
	slot.owner = owner
	slot.type_name = type_name
	slot.owned = []u32{}
	slot.reads = []u32{}
	slot.readers = map[u32]bool{}
	slot.render = unsafe { nil }
	slot.window = 0
	slot.dirty = true
	slot.cached = none
	if owner != 0 {
		s.slots[owner].owned << id
	}
	return id, slot.gen
}

fn (s &Store) live(id u32, gen u32) bool {
	return id > 0 && int(id) < s.slots.len && s.slots[id].alive && s.slots[id].gen == gen
}

fn (s &Store) ptr(id u32, gen u32, what string) voidptr {
	if !s.live(id, gen) {
		panic('quadra: stale ${what} handle ${id}#${gen}')
	}
	return s.slots[id].ptr
}

pub fn (app &App) is_alive(id u32, gen u32) bool {
	return app.store.live(id, gen)
}

pub fn (app &App) model_count() int {
	mut n := 0
	for s in app.store.slots {
		if s.alive {
			n++
		}
	}
	return n
}

// heap_copy moves a value onto the GC heap (scanned, so it may hold strings and arrays).
fn heap_copy[T](value T) voidptr {
	p := unsafe { malloc(int(sizeof(T))) }
	unsafe { vmemcpy(p, &value, int(sizeof(T))) }
	return p
}

// release frees a model and, depth-first, everything it owns, with its subscriptions.
fn (mut app App) release_id(id u32) {
	if id == 0 || int(id) >= app.store.slots.len || !app.store.slots[id].alive {
		return
	}
	owned := app.store.slots[id].owned.clone()
	for child in owned {
		app.release_id(child)
	}
	app.drop_subscriptions(id)
	app.cancel_owned_timers(id)
	app.cancel_owned_tasks(id)
	owner := app.store.slots[id].owner
	if owner != 0 {
		app.store.slots[owner].owned = app.store.slots[owner].owned.filter(it != id)
	}
	app.retire_life(id)
	mut slot := &app.store.slots[id]
	slot.alive = false
	slot.ptr = unsafe { nil }
	slot.readers = map[u32]bool{}
	slot.reads = []u32{}
	slot.cached = none
	app.store.free << id
}

// track records that the view being rendered read model `id`.
fn (mut app App) track(id u32) {
	if app.rendering.len == 0 {
		return
	}
	view := app.rendering.last()
	if view == id {
		return
	}
	if !app.store.slots[id].readers[view] {
		app.store.slots[id].readers[view] = true
		app.store.slots[view].reads << id
	}
}

fn (mut app App) begin_tracking(view u32) {
	// Forget last frame's reads: dependencies are exactly what this render reads.
	for m in app.store.slots[view].reads {
		if app.store.slots[m].alive {
			app.store.slots[m].readers.delete(view)
		}
	}
	app.store.slots[view].reads = []u32{}
	app.rendering << view
}

fn (mut app App) end_tracking() {
	app.rendering.pop()
}

// store_ptr gives the raw value of a live model (escape hatch for tests and bridges).
pub fn (app &App) store_ptr(id u32) voidptr {
	return app.store.slots[id].ptr
}

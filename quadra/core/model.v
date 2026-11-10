// Models: app-owned state with an ownership tree and automatic dependency tracking
// (specs/07-state.md). Handles are plain values; stale handles are detected.
module core

import quadra.core.life
import quadra.prim

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
	// views: render arenas (arena.v): the latest render's, the one it replaced in this
	// build, the free one; arena_build: a render of this build took an arena; arena_pin:
	// something may hold elements of the view's arenas (App.escaped)
	arena       &Arena = unsafe { nil }
	arena_old   &Arena = unsafe { nil }
	arena_free  &Arena = unsafe { nil }
	arena_build bool
	arena_pin   bool
	// views: the last layout placement (relayout.v): its element, window, full-layout epoch
	// and pass, the render it laid out; multi: placed twice in one pass
	placed        &ViewElement = unsafe { nil }
	placed_win    u32
	placed_epoch  u32
	placed_build  int
	placed_render u32
	renders       u32
	multi         bool
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
	slot.placed = unsafe { nil }
	slot.placed_epoch = 0
	if owner != 0 {
		s.slots[owner].owned << id
	}
	return id, slot.gen
}

fn (s &Store) live(id u32, gen u32) bool {
	return id > 0 && int(id) < s.slots.len && s.slots[id].alive && s.slots[id].gen == gen
}

fn (s &Store) ptr(id u32, gen u32, what string) prim.Holder {
	if !s.live(id, gen) {
		panic('quadra: stale ${what} handle ${id}#${gen}')
	}
	return s.holder(id)
}

// holder gives slot id's value as a checked holder (slot 0: empty), tagged with the type the
// slot was made for: a Model[T] or listener of another type panics outside -prod.
@[inline]
fn (s &Store) holder(id u32) prim.Holder {
	mut name := ''
	$if !prod {
		name = s.slots[id].type_name
	}
	return prim.hold_named(s.slots[id].ptr, name)
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

// heap_hold is heap_copy in a checked holder.
fn heap_hold[T](value T) prim.Holder {
	return prim.hold_named(heap_copy(value), typeof[T]().name)
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
	slot.placed = unsafe { nil }
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

// model_holder gives a live model's value as a checked holder, for type-erased code that
// reads it back with prim.held (app.Settings).
pub fn (app &App) model_holder(id u32, gen u32) prim.Holder {
	return app.store.ptr(id, gen, 'model')
}

// self_model is the handle of the model this context acts for.
pub fn (cx &ViewCx[T]) self_model() Model[T] {
	return Model[T]{cx.entity, cx.app.store.slots[cx.entity].gen}
}

// Element arenas (specs/07, Element memory): divs and texts built by a view's render, or by
// a kit component's build during layout, come from a pool used before, so a frame that
// rebuilds them reuses the elements of the build before last instead of leaving them to the
// collector.
//
// A view has an arena for the tree of its latest render (its cache) and a free one. A render
// fills the free one; at the end of the build in which it rendered, the arena of the replaced
// tree is reset and becomes the free one, at the moment the replaced render's closures are
// freed (tracked.v). A window's layout pass works the same way with the trees kit components
// build (Lazy): the previous pass's arena is reset at the end of the next one. A pin (an
// escape: a model, a global, a popup, a drag preview, keep_build) drops arenas to the
// collector instead. Handlers, prepaint builds (list rows, overlays) and tests use the heap.
module core

// Arena is a pool of elements. quadra.ui owns the element types: a pool holds pointers to its
// elements (kinds: 0 divs, 1 texts, 2 svgs), `used` counts those the current render took.
@[heap]
pub struct Arena {
pub mut:
	pools [3][]voidptr
	used  [3]int
}

pub type ArenaReset = fn (mut a Arena)

// ArenaUse is the arena being filled and who owns it: a view (owner) or a window's layout
// pass (pin points at its pin flag). An escape pins the owner.
struct ArenaUse {
	arena &Arena = unsafe { nil }
	owner u32
	pin   &bool = unsafe { nil }
}

struct ArenaState {
mut:
	cur   ArenaUse
	reset ArenaReset = unsafe { nil } // set by quadra.ui: resets the elements an arena handed out
}

// One per process: elements are built on the main thread only.
const arena_state = &ArenaState{}

// render_arena is the arena being filled, or nil: build on the heap.
@[inline]
pub fn render_arena() &Arena {
	return arena_state.cur.arena
}

// set_arena_reset installs the function that resets used elements (quadra.ui, at start-up).
pub fn set_arena_reset(f ArenaReset) {
	mut st := unsafe { arena_state }
	st.reset = f
}

fn use_arena(u ArenaUse) ArenaUse {
	mut st := unsafe { arena_state }
	old := st.cur
	st.cur = u
	return old
}

fn reset_arena(mut a Arena) {
	if arena_state.reset != unsafe { nil } {
		arena_state.reset(mut a)
	}
	a.used = [0, 0, 0]!
}

// heap_render renders a view with its elements on the heap.
fn heap_render(mut app App, win &Window, id u32, render RenderFn) Element {
	saved := use_arena(ArenaUse{})
	el := render(mut app, win, id)
	use_arena(saved)
	return el
}

// nested_render renders a view reached while another render or a kit build is running: in
// the view's own arena, without a closure frame of its own (tracked.v).
fn (mut app App) nested_render(id u32, win &Window, render RenderFn) Element {
	a := app.take_arena(id)
	if a == unsafe { nil } {
		return heap_render(mut app, win, id, render)
	}
	saved := use_arena(ArenaUse{ arena: a, owner: id })
	el := render(mut app, win, id)
	use_arena(saved)
	app.put_arena(id, a, win)
	return el
}

// take_arena returns the free arena for a render of view id, or nil when the view already
// rendered in this build (the tree of that render may still be placed in the window).
fn (mut app App) take_arena(id u32) &Arena {
	mut s := &app.store.slots[id]
	if s.arena_build {
		return unsafe { nil }
	}
	mut a := s.arena_free
	s.arena_free = unsafe { nil }
	if a == unsafe { nil } {
		a = &Arena{}
	}
	return a
}

// put_arena records that view id's latest render was built in a; the window settles it.
fn (mut app App) put_arena(id u32, a &Arena, win &Window) {
	if a == unsafe { nil } {
		return
	}
	mut s := &app.store.slots[id]
	s.arena_old = s.arena
	s.arena = unsafe { a }
	s.arena_build = true
	mut w := unsafe { win }
	w.tracking.arena_views << id
}

// give_back returns an arena a render did not use.
fn (mut app App) give_back(id u32, a &Arena) {
	if a != unsafe { nil } {
		app.store.slots[id].arena_free = unsafe { a }
	}
}

// settle_arenas runs at the end of a build, for the views that took an arena since the last
// one: the replaced trees' arenas are reset and free again; a pinned view drops both.
fn (mut app App) settle_arenas(mut w Window) {
	for id in w.tracking.arena_views {
		if int(id) >= app.store.slots.len || !app.store.slots[id].alive {
			continue
		}
		mut s := &app.store.slots[id]
		s.arena_build = false
		if s.arena_pin {
			s.arena_pin = false
			s.arena = unsafe { nil }
			s.arena_old = unsafe { nil }
			continue
		}
		mut old := s.arena_old
		s.arena_old = unsafe { nil }
		if old != unsafe { nil } {
			reset_arena(mut old)
			if s.arena_free == unsafe { nil } {
				s.arena_free = old
			}
		}
	}
	w.tracking.arena_views.clear()
}

// pin_arenas is the arena side of App.escaped: during a render or build, the arena being
// filled and those of the views rendered so far in the build (a nested view's tree is part
// of what escaped); from a handler or keep_build, every arena that holds a tree (all).
fn (mut app App) pin_arenas(all bool) {
	cur := arena_state.cur
	if cur.owner != 0 && int(cur.owner) < app.store.slots.len {
		app.store.slots[cur.owner].arena_pin = true
	}
	if cur.pin != unsafe { nil } {
		mut pin := unsafe { cur.pin }
		unsafe {
			*pin = true
		}
	}
	if !all {
		for w in app.windows {
			for id in w.tracking.arena_views {
				if int(id) < app.store.slots.len {
					app.store.slots[id].arena_pin = true
				}
			}
		}
		return
	}
	for mut s in app.store.slots {
		if s.alive && (s.arena != unsafe { nil } || s.arena_old != unsafe { nil }) {
			s.arena_pin = true
		}
	}
	for mut w in app.windows {
		w.tracking.pass.pin = true
	}
}

// drop_arenas forgets a released view's arenas: its last tree stays on screen until its
// window rebuilds, and the collector frees it after that.
fn drop_arenas(mut s Slot) {
	s.arena = unsafe { nil }
	s.arena_old = unsafe { nil }
	s.arena_free = unsafe { nil }
	s.arena_build = false
	s.arena_pin = false
}

// PassArenas are a window's arenas for the trees kit components build during layout: the
// current pass's, the previous pass's (still placed until this pass ends) and a free one.
struct PassArenas {
mut:
	cur  &Arena = unsafe { nil }
	old  &Arena = unsafe { nil }
	free &Arena = unsafe { nil }
	pin  bool
}

// pass_arena is the arena kit builds of the current layout pass fill.
fn (mut p PassArenas) take() ArenaUse {
	if p.cur == unsafe { nil } {
		p.cur = if p.free != unsafe { nil } { p.free } else { &Arena{} }
		p.free = unsafe { nil }
	}
	return ArenaUse{
		arena: p.cur
		pin:   &p.pin
	}
}

// settle ends a layout pass: the previous pass's trees are gone, so its arena is reset and
// free; kept (keep_build) or pinned drops both instead (an overlay holds what they built).
fn (mut p PassArenas) settle(kept bool) {
	if kept || p.pin {
		p.cur, p.old, p.pin = unsafe { nil }, unsafe { nil }, false
		return
	}
	mut old := p.old
	p.old, p.cur = p.cur, unsafe { nil }
	if old != unsafe { nil } {
		reset_arena(mut old)
		if p.free == unsafe { nil } {
			p.free = old
		}
	}
}

// Render arenas (specs/07, Element memory): elements built by a view's render come from a
// pool the view used before, so a view that renders every frame reuses the elements of its
// render before last instead of leaving them to the collector.
//
// A view has an arena for the tree of its latest render (its cache) and a free one. A render
// fills the free one; at the end of the build that rendered it, the arena of the replaced
// tree is reset and becomes the free one, at the same moment the replaced render's closures
// are freed (tracked.v). A pinned render drops both arenas instead. Everything outside a
// tracked render (handlers, layout and prepaint builds, tests) builds on the heap.
module core

// Arena is a pool of elements. quadra.ui owns the element types: a pool holds pointers to its
// elements (kind 0 divs, 1 texts), `used` counts those the current render took.
@[heap]
pub struct Arena {
pub mut:
	pools [2][]voidptr
	used  [2]int
}

pub type ArenaReset = fn (mut a Arena)

struct ArenaState {
mut:
	current &Arena     = unsafe { nil }
	reset   ArenaReset = unsafe { nil } // set by quadra.ui: resets the elements an arena handed out
}

// One per process: elements are built on the main thread only.
const arena_state = &ArenaState{}

// render_arena is the arena of the render in progress, or nil: build on the heap.
@[inline]
pub fn render_arena() &Arena {
	return arena_state.current
}

// set_arena_reset installs the function that resets used elements (quadra.ui, at start-up).
pub fn set_arena_reset(f ArenaReset) {
	mut st := unsafe { arena_state }
	st.reset = f
}

fn set_render_arena(a &Arena) &Arena {
	mut st := unsafe { arena_state }
	old := st.current
	st.current = unsafe { a }
	return old
}

// heap_render renders a view with its elements on the heap (nested or untracked renders).
fn heap_render(mut app App, win &Window, id u32, render RenderFn) Element {
	saved := set_render_arena(unsafe { nil })
	el := render(mut app, win, id)
	set_render_arena(saved)
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

// put_arena records that view id's latest render was built in a (nil: on the heap).
fn (mut app App) put_arena(id u32, a &Arena) {
	if a == unsafe { nil } {
		return
	}
	mut s := &app.store.slots[id]
	s.arena_old = s.arena
	s.arena = unsafe { a }
	s.arena_build = true
}

// give_back returns an arena a render did not use.
fn (mut app App) give_back(id u32, a &Arena) {
	if a != unsafe { nil } {
		app.store.slots[id].arena_free = unsafe { a }
	}
}

// settle_arena runs at the end of a build that rendered the view: the replaced tree's arena
// is reset and free again, or, for a pinned render, both arenas are left to the collector.
fn settle_arena(mut s Slot, pinned bool) {
	s.arena_build = false
	if pinned {
		s.arena = unsafe { nil }
		s.arena_old = unsafe { nil }
		return
	}
	mut old := s.arena_old
	s.arena_old = unsafe { nil }
	if old == unsafe { nil } {
		return
	}
	if arena_state.reset != unsafe { nil } {
		arena_state.reset(mut old)
	}
	old.used = [0, 0]!
	if s.arena_free == unsafe { nil } {
		s.arena_free = old
	}
}

// drop_arenas forgets a released view's arenas: its last tree stays on screen until its
// window rebuilds, and the collector frees it after that.
fn drop_arenas(mut s Slot) {
	s.arena = unsafe { nil }
	s.arena_old = unsafe { nil }
	s.arena_free = unsafe { nil }
	s.arena_build = false
}

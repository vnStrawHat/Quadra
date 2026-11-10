// Closure lifetimes for elements built during layout and prepaint.
//
// V never frees a closure on its own: each `fn [..]` evaluation keeps its slot and captured
// values forever unless it was created inside a `closure.Lifetime` frame. Kit components
// rebuild their element trees at every layout, and virtual lists rebuild visible rows at
// every prepaint, so without this every keystroke and scroll step would leak.
//
// `tracked` runs a builder inside the window's layout or prepaint lifetime. What it builds is
// dead once the next pass of the same kind has finished (the tree is rebuilt), so each pass
// reclaims the frames of the passes before it.
module core

import quadra.core.life

// Builder produces an element tree. It must only build elements: closures it creates are
// freed after the next layout (or prepaint), so they must not be stored anywhere else.
pub interface Builder {
mut:
	build_tree(mut cx LayoutCx) Element
}

pub enum Phase {
	idle
	layout
	prepaint
}

struct Tracking {
mut:
	layout   &life.Life = life.new()
	prepaint &life.Life = life.new()
	frames   [3]int       // frames opened in the current pass, by Phase
	pinned   [3]bool      // keep_build: keep the current pass's lifetime, by Phase
	kept     []&life.Life // lifetimes kept by keep_build until free_kept
	free     bool         // free_kept: dispose `kept` at the end of the next layout
	phase    Phase
	active   bool
	// what the frame's work function (run_build) builds, with `cx`
	pending ?Builder
	cx      &LayoutCx = unsafe { nil }
	out     ?Element
	// views rendered by this build (their previous renders are freed at its end)
	rendered []u32
	// element arenas (arena.v): views that took one since the last build, the layout pass's
	arena_views []u32
	pass        PassArenas
}

// tracked builds b's tree; closures created while building belong to the current pass.
// Outside layout and prepaint (or nested) it just builds.
pub fn tracked(mut b Builder, mut cx LayoutCx) Element {
	if cx.win == unsafe { nil } {
		return b.build_tree(mut cx)
	}
	mut w := cx.win
	mut t := &w.tracking
	if t.phase == .idle || t.active {
		return b.build_tree(mut cx)
	}
	t.pending = b
	t.cx = unsafe { &cx }
	t.out = none
	t.active = true
	mut lt := if t.phase == .layout { t.layout } else { t.prepaint }
	// Layout builds fill the pass arena; prepaint builds (rows, overlays) use the heap.
	mut u := ArenaUse{}
	if t.phase == .layout {
		u = t.pass.take()
	}
	saved := use_arena(u)
	mut cur := unsafe { frame_work }
	cur.t = t
	lt.frame(run_build) or {
		use_arena(saved)
		t.active = false
		return b.build_tree(mut cx)
	}
	use_arena(saved)
	t.active = false
	t.frames[int(t.phase)]++
	t.pending = none
	return t.out or { b.build_tree(mut cx) }
}

// begin_pass starts a layout or prepaint pass.
fn (mut w Window) begin_pass(p Phase) {
	w.tracking.phase = p
	w.tracking.frames[int(p)] = 0
}

// end_pass frees what earlier passes of this kind built: their elements were replaced.
fn (mut w Window) end_pass(p Phase) {
	mut t := &w.tracking
	t.phase = .idle
	keep := t.frames[int(p)]
	pinned := t.pinned[int(p)]
	t.pinned[int(p)] = false
	if p == .layout && t.free {
		for mut l in t.kept {
			l.dispose()
		}
		t.kept.clear()
		t.free = false
	}
	if p == .layout {
		t.pass.settle(pinned)
		if pinned {
			t.kept << t.layout // an overlay holds closures it built
			t.layout = life.new()
		} else {
			t.layout.reclaim(keep)
		}
		mut app := w.app
		app.settle_renders(mut w)
	} else if pinned {
		t.kept << t.prepaint
		t.prepaint = life.new()
	} else {
		t.prepaint.reclaim(keep)
	}
}

// release_tracking frees everything a closed window built.
fn (mut w Window) release_tracking() {
	w.tracking.layout.dispose()
	w.tracking.prepaint.dispose()
	for mut l in w.tracking.kept {
		l.dispose()
	}
	w.tracking.kept.clear()
}

// --- Render lifetimes ------------------------------------------------------------------
//
// Each view's render runs in a frame of the view's own lifetime. When the view renders again,
// the previous render's closures are freed at the end of that build (its elements were
// replaced). A render whose closures may have escaped into something long-lived (a model,
// a global, an observer, a timer...) is pinned: its lifetime is abandoned instead, so what
// escaped stays valid. Rule for app code: a closure made during render belongs to that
// render; make long-lived ones in constructors or handlers.

struct RenderJob {
mut:
	app    &App    = unsafe { nil }
	win    &Window = unsafe { nil }
	id     u32
	render RenderFn = unsafe { nil }
	out    ?Element
	active u32 // view whose render frame is open
}

struct DeadLife {
	lifetime &life.Life
	win      u32
}

fn (mut app App) render_tracked(id u32, win &Window, render RenderFn) Element {
	mut j := &app.rjob
	if j.active != 0 || win.tracking.active {
		return app.nested_render(id, win, render)
	}
	if app.store.slots[id].lifetime == unsafe { nil } {
		app.store.slots[id].lifetime = life.new()
	}
	mut lt := app.store.slots[id].lifetime // the slots array may grow while rendering
	j.app = app
	j.win = win
	j.id = id
	j.render = render
	j.out = none
	j.active = id
	arena := app.take_arena(id)
	saved := use_arena(ArenaUse{ arena: arena, owner: id })
	mut cur := unsafe { frame_work }
	cur.j = j
	lt.frame(run_render) or {
		use_arena(saved)
		app.give_back(id, arena)
		j.active = 0
		return heap_render(mut app, win, id, render)
	}
	use_arena(saved)
	app.put_arena(id, arena, win)
	j.active = 0
	mut w := unsafe { win }
	w.tracking.rendered << id
	return j.out or { heap_render(mut app, win, id, render) }
}

// escaped is called by every registry that keeps a value beyond the current render. During
// a render it pins that render. From a handler, what is stored may capture closures of any
// current render, so every view is pinned (rare: opening popups, creating models).
fn (mut app App) escaped() {
	if app.rjob.active != 0 {
		app.store.slots[app.rjob.active].pinned = true
		app.pin_arenas(false)
		return
	}
	for mut s in app.store.slots {
		if s.alive && s.lifetime != unsafe { nil } {
			s.pinned = true
		}
	}
	app.pin_arenas(true)
}

// keep_build keeps what the window's latest layout and prepaint passes built until free_kept.
// An overlay opened from a kit component (built in layout) or a list row (built in prepaint)
// holds closures of that build: a tab strip's overflow menu items.
pub fn (mut cx Cx) keep_build() {
	if cx.win != unsafe { nil } {
		mut w := cx.win
		w.tracking.pinned = [true, true, true]!
	}
	if cx.app != unsafe { nil } {
		mut app := cx.app
		app.pin_arenas(true) // views rendered inside those builds hold part of the kept trees
	}
}

// free_kept frees what keep_build kept for window win, after its next layout (the handler
// that closed the overlay may still be running one of those closures).
pub fn (mut app App) free_kept(win u32) {
	for mut w in app.windows {
		if w.id == win && w.tracking.kept.len > 0 {
			w.tracking.free = true
		}
	}
}

// escaped_in_render pins the current render, if any. For registries fed mostly from
// handlers with closures made there (timers, debounces, tasks): pinning every view on each
// keystroke would keep every render alive.
fn (mut app App) escaped_in_render() {
	if app.rjob.active != 0 {
		app.store.slots[app.rjob.active].pinned = true
	}
	app.pin_arenas(false)
}

// settle_renders frees the previous renders of the views this build rendered.
fn (mut app App) settle_renders(mut w Window) {
	app.settle_arenas(mut w)
	for id in w.tracking.rendered {
		if int(id) >= app.store.slots.len || !app.store.slots[id].alive {
			continue
		}
		mut s := &app.store.slots[id]
		if s.lifetime == unsafe { nil } {
			continue
		}
		if s.pinned {
			s.lifetime = life.new() // abandoned: what escaped stays valid
			s.pinned = false
		} else {
			s.lifetime.reclaim(1)
		}
	}
	w.tracking.rendered.clear()
	// Released views: their last elements were on screen until this build replaced them.
	// Views of closed windows go at the next build of any window (no handler of a closed
	// window can be running then).
	mut keep := []DeadLife{}
	for d in app.dead_lives {
		if d.win == w.id || d.win == 0 || !app.windows.any(it.id == d.win) {
			mut l := d.lifetime
			l.dispose()
		} else {
			keep << d
		}
	}
	app.dead_lives = keep
}

// retire_life schedules a released view's closures to be freed after its window rebuilds.
fn (mut app App) retire_life(id u32) {
	mut s := &app.store.slots[id]
	if s.lifetime != unsafe { nil } && !s.pinned {
		app.dead_lives << DeadLife{s.lifetime, s.window}
	}
	s.lifetime = unsafe { nil }
	s.pinned = false
	drop_arenas(mut s)
}

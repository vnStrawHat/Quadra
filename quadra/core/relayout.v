// Incremental builds (specs/07, Invalidation levels): when only views were notified, the
// window renders those views and lays out what their new trees change, keeping the rest of
// the layout tree. Each view's placement remembers its layout node; the view's new subtree
// replaces the old one (layout.Tree.replace). Everything else (invalidate, resize, scale,
// theme) lays out the whole window again.
module core

import os
import time
import quadra.prim
import quadra.layout
import quadra.text
import quadra.core.life

// QUADRA_VERIFY_LAYOUT=1 checks every incremental layout against a full one (slow).
const verify_layout = os.getenv('QUADRA_VERIFY_LAYOUT') != ''

// Relayout is a window's incremental-build state.
struct Relayout {
mut:
	full   bool = true // the next build lays out everything
	views  []u32 // views notified since the last build
	epoch  u32   // counts full layouts: placements made before the last one are void
	build  int   // counts layout passes
	active bool  // an incremental pass is running: clean nested views keep their nodes
	size   prim.Size
	scale  f32
	ts     text.TextStyle
	live   int // layout nodes after the last full layout (replaced subtrees stay behind)
	frames int // closure frames kit builds opened in incremental passes since then
}

// ViewElement places a child view; it renders the view when laid out.
pub struct ViewElement {
	view AnyView
mut:
	inner ?Element
	fresh int // layout pass in which the view last rendered (paint flashing)
	node  layout.NodeId
	ts    text.TextStyle // inherited text style at the placement
	depth int            // views above it
}

pub fn (mut e ViewElement) request_layout(mut cx LayoutCx) layout.NodeId {
	mut app := cx.app
	mut w := cx.win
	id := e.view.id
	before := w.stats.renders[id]
	mut el := app.render_any(id, w)
	rendered := w.stats.renders[id] != before
	if rendered {
		e.fresh = w.relay.build
	}
	e.inner = el
	if w.tracking.phase != .layout {
		return el.request_layout(mut cx) // rows and overlays laid out in prepaint: scratch nodes
	}
	s := &app.store.slots[id]
	old := s.placed
	if w.relay.active && old != unsafe { nil } && s.placed_render == s.renders
		&& s.placed_win == w.id && s.placed_epoch == w.relay.epoch && !s.multi
		&& s.placed_build != w.relay.build && old.ts == cx.text_style {
		// A clean view placed again by a parent that rendered: its subtree is unchanged.
		e.node = w.tree.current(old.node)
		e.fresh, e.ts, e.depth = old.fresh, old.ts, cx.views
	} else {
		e.ts, e.depth = cx.text_style, cx.views
		mut t := cx.tree
		owner := t.owner
		t.owner = id
		cx.views++
		e.node = el.request_layout(mut cx)
		cx.views--
		t.owner = owner
	}
	app.place(id, mut e, w)
	return e.node
}

// place records where the layout put view id (the slots may have grown while it rendered).
fn (mut app App) place(id u32, mut e ViewElement, w &Window) {
	mut s := &app.store.slots[id]
	same_epoch := s.placed_win == w.id && s.placed_epoch == w.relay.epoch
	if same_epoch && s.placed_build == w.relay.build {
		s.multi = true // placed twice: an incremental pass could not tell which to update
	} else if !same_epoch {
		s.multi = false
	}
	s.placed = unsafe { &e }
	s.placed_win, s.placed_epoch, s.placed_build = w.id, w.relay.epoch, w.relay.build
	s.placed_render = s.renders
	if s.window == 0 {
		s.window = w.id
	}
}

pub fn (mut e ViewElement) prepaint(bounds prim.Rect, mut cx PrepaintCx) {
	if e.fresh == cx.win.relay.build {
		mut w := cx.win
		w.fresh_views << bounds
	}
	cx.enter('v${e.view.id}')
	if mut el := e.inner {
		el.prepaint(bounds, mut cx)
	}
	cx.leave()
}

pub fn (mut e ViewElement) paint(bounds prim.Rect, mut cx PaintCx) {
	if mut el := e.inner {
		el.paint(bounds, mut cx)
	}
}

// notify_view schedules a build for a notified view of this window.
fn (mut w Window) notify_view(id u32) {
	w.relay.views << id
	w.dirty = true
	w.pw.request_frame()
}

// layout_all lays out the whole window from root, the root view's tree.
fn (mut w Window) layout_all(mut root Element) {
	mut app := w.app
	mut r := &w.relay
	r.epoch++
	r.build++
	w.tree.clear()
	mut lcx := LayoutCx{
		app:        app
		win:        w
		tree:       &w.tree
		text:       app.text
		text_style: app.base_text_style()
	}
	w.begin_pass(.layout)
	w.tree.owner = w.root.id
	node := root.request_layout(mut lcx)
	w.tracking.phase = .idle
	w.tree.compute_new(node, w.pw.size())
	w.root_el = root
	w.root_node = node
	w.layout_len = w.tree.len()
	r.size, r.scale, r.ts = w.pw.size(), w.pw.scale(), lcx.text_style
	r.live, r.frames = w.layout_len, 0
}

// verify_layout compares the window's layout with a full one from scratch: the first
// difference, or none (tests).
pub fn (w &Window) verify_layout() ?string {
	return w.tree.verify(w.root_node)
}

struct RelayoutJob {
	id    u32
	depth int
}

// relayout renders and lays out only the views in `views`, deepest first (a parent's new
// tree then places its children's new subtrees); none: the window needs a full layout.
// It returns the microseconds spent rendering.
fn (mut w Window) relayout(views []u32, mut app App) ?i64 {
	mut r := &w.relay
	if w.root_el == none || w.pw.size() != r.size || w.pw.scale() != r.scale
		|| app.base_text_style() != r.ts || w.tracking.free || w.tree.len() > 2 * r.live + 4096
		|| r.frames > 256 {
		return none
	}
	mut jobs := []RelayoutJob{cap: views.len}
	for id in views {
		if int(id) >= app.store.slots.len {
			continue
		}
		s := &app.store.slots[id]
		if !s.alive || !s.dirty {
			continue
		}
		if id == w.root.id {
			jobs << RelayoutJob{id, -1}
			continue
		}
		// Not placed by the last layouts: not in the tree, or placed by prepaint (list
		// rows, overlays), which renders it below anyway.
		if s.placed == unsafe { nil } || s.placed_win != w.id || s.placed_epoch != r.epoch {
			continue
		}
		if s.multi || !w.tree.attached(w.tree.current(s.placed.node), w.root_node) {
			return none
		}
		jobs << RelayoutJob{id, s.placed.depth}
	}
	jobs.sort(a.depth > b.depth)
	w.tree.truncate(w.layout_len)
	w.tree.measure_calls = 0
	w.tree.climb_work = 0
	w.stats.relaid = 0
	r.build++
	r.active = true
	w.begin_pass(.layout)
	mut render_us := i64(0)
	for job in jobs {
		if app.store.slots[job.id].dirty {
			render_us += w.relayout_view(job.id, mut app)
		}
		if w.tree.climb_work > r.live {
			// Many changes that each reach a long list: one full layout is cheaper. The views
			// rendered so far keep their new trees; it places them.
			w.tracking.phase = .idle
			r.active = false
			return none
		}
	}
	w.tracking.phase = .idle
	r.active = false
	w.layout_len = w.tree.len()
	w.stats.relayouts++
	if verify_layout {
		if d := w.tree.verify(w.root_node) {
			panic('quadra: incremental layout differs from a full one: ${d}')
		}
	}
	return render_us
}

// relayout_view renders view id and puts its new subtree in place of the old one.
fn (mut w Window) relayout_view(id u32, mut app App) i64 {
	is_root := id == w.root.id
	mut e := app.store.slots[id].placed
	old := if is_root { w.root_node } else { w.tree.current(e.node) }
	sw := time.new_stopwatch()
	mut el := app.render_any(id, w)
	us := sw.elapsed().microseconds()
	mut lcx := LayoutCx{
		app:        app
		win:        w
		tree:       &w.tree
		text:       app.text
		text_style: app.base_text_style()
	}
	if !is_root {
		e.inner, e.fresh = el, w.relay.build
		lcx.text_style, lcx.views = e.ts, e.depth + 1
	}
	w.tree.owner = id
	w.tree.recycle_below(old, id) // the old tree's own nodes, filled again where equal
	fresh := el.request_layout(mut lcx)
	w.tree.end_recycle()
	if is_root {
		w.root_el = el
	} else {
		e.node = fresh
		app.store.slots[id].placed_render = app.store.slots[id].renders
	}
	w.stats.relaid += w.tree.replace(old, fresh)
	if old == w.root_node {
		w.root_node = fresh
	}
	return us
}

// end_relayout ends an incremental layout pass. The kit trees of clean views are still
// placed, so nothing earlier passes built is freed; the next full layout frees it all
// (Relayout.frames forces one before many frames pile up).
fn (mut w Window) end_relayout() {
	mut t := &w.tracking
	t.phase = .idle
	w.relay.frames += t.frames[int(Phase.layout)]
	if t.pinned[int(Phase.layout)] {
		t.pinned[int(Phase.layout)] = false
		t.pass.settle(true)
		t.kept << t.layout // an overlay holds closures it built
		t.layout = life.new()
	}
	mut app := w.app
	app.settle_renders(mut w)
}

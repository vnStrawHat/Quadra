// UniformList: a virtualized list of equal-height rows. Only visible rows are built,
// laid out and painted, during prepaint, so a million rows cost one screenful.
module ui

import math
import quadra.core
import quadra.prim
import quadra.layout

pub type RowFn = fn (i int) core.Element

pub struct ListState {
pub mut:
	y f32
mut:
	heights Heights // variable-height lists only
}

struct ListScroll {
mut:
	state &ListState = unsafe { nil }
	max_y f32
}

struct VisibleRow {
mut:
	el     core.Element
	bounds prim.Rect
}

@[heap]
pub struct UniformList {
pub mut:
	id     string
	count  int
	row_h  f32
	reveal int = -1 // scroll so this row is visible (e.g. keyboard selection)
	set_y  f32 = -1 // scroll to this offset (a scroll position the app controls)
	style  layout.Style
mut:
	build  core.Bound[int, core.Element]
	node   layout.NodeId
	rows   []VisibleRow
	scroll ListScroll
	offset f32
}

// uniform_list fills its parent by default; render(i) builds row i on demand.
pub fn uniform_list(id string, count int, row_h f32, render RowFn) &UniformList {
	return &UniformList{
		id:    id
		count: count
		row_h: row_h
		build: row_bound(render)
		style: layout.Style{
			grow:   1
			width:  layout.pct(100)
			height: layout.pct(100)
		}
	}
}

// uniform_list_with is uniform_list with the data the row builder needs passed in, so the
// builder captures nothing (specs/14). D must not be a pointer type.
pub fn uniform_list_with[D](id string, count int, row_h f32, data D, render fn (d D, i int) core.Element) &UniformList {
	return &UniformList{
		id:    id
		count: count
		row_h: row_h
		build: core.bind_fn[D, int, core.Element](data, render)
		style: layout.Style{
			grow:   1
			width:  layout.pct(100)
			height: layout.pct(100)
		}
	}
}

pub fn (self_ &UniformList) height(h f32) &UniformList {
	mut l := unsafe { self_ }
	l.style.height = layout.px(h)
	l.style.grow = 0
	return l
}

// scroll_y sets the scroll offset, as a scroll position owned by the app.
pub fn (self_ &UniformList) scroll_y(y f32) &UniformList {
	mut l := unsafe { self_ }
	l.set_y = y
	return l
}

pub fn (self_ &UniformList) reveal(i int) &UniformList {
	mut l := unsafe { self_ }
	l.reveal = i
	return l
}

pub fn (mut l UniformList) request_layout(mut cx core.LayoutCx) layout.NodeId {
	l.node = cx.tree.node(l.style, [])
	return l.node
}

fn list_scroll(holder prim.Holder, _ prim.Holder, event prim.Holder, mut cx core.Cx) {
	mut s := prim.held[ListScroll](holder)
	e := prim.held[core.ScrollEvent](event)
	ny := prim.clamp(s.state.y - e.dy, 0, s.max_y)
	if ny != s.state.y {
		s.state.y = ny
		cx.win.invalidate_prepaint()
		cx.stop_propagation()
	}
}

pub fn (mut l UniformList) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter(l.id)
	defer {
		cx.leave()
	}
	mut st := cx.state(ListState{})
	total := f32(l.count) * l.row_h
	max_y := prim.max(0, total - bounds.height)
	if l.set_y >= 0 {
		st.y = l.set_y
	}
	if l.reveal >= 0 && l.reveal < l.count {
		top := f32(l.reveal) * l.row_h
		if top < st.y {
			st.y = top
		} else if top + l.row_h > st.y + bounds.height {
			st.y = top + l.row_h - bounds.height
		}
		l.reveal = -1
	}
	st.y = prim.clamp(st.y, 0, max_y)
	l.offset = st.y
	l.scroll = ListScroll{st, max_y}
	hb := cx.insert_hitbox(bounds, core.HitboxOptions{})
	cx.on_mouse(hb, .scroll, core.AnyListener{
		holder: prim.hold(&l.scroll)
		call:   list_scroll
	}, false)
	cx.push_hitbox(hb)
	cx.push_clip(bounds)
	l.rows.clear()
	if l.row_h > 0 && l.count > 0 {
		first := int(st.y / l.row_h)
		last := int(math.min(l.count, first + int(math.ceil(bounds.height / l.row_h)) + 1))
		mut lcx := cx.layout_cx()
		for i in first .. last {
			mut rb := RowBuild{l.build, i}
			mut el := core.tracked(mut rb, mut lcx) // rows die at the next prepaint
			node := el.request_layout(mut lcx)
			cx.tree.compute_fixed(node, bounds.width, l.row_h)
			r := prim.rect(bounds.x, bounds.y + f32(i) * l.row_h - st.y, bounds.width, l.row_h)
			el.prepaint(r, mut cx)
			l.rows << VisibleRow{el, r}
		}
	}
	cx.pop_clip()
	cx.pop_hitbox()
}

pub fn (mut l UniformList) paint(bounds prim.Rect, mut cx core.PaintCx) {
	cx.push_clip(bounds, 0)
	for mut r in l.rows {
		r.el.paint(r.bounds, mut cx)
	}
	cx.pop_clip()
	cx.scroll_thumb(bounds, f32(l.count) * l.row_h, l.offset, true, core.ThumbTarget{prim.hold(l.scroll.state), set_list_scroll})
}

fn set_list_scroll(holder prim.Holder, offset f32, _ bool) {
	mut st := prim.held[ListState](holder)
	st.y = offset
}

// RowBuild builds one row inside the window's prepaint lifetime (core.tracked).
struct RowBuild {
	build core.Bound[int, core.Element]
	i     int
}

fn (mut r RowBuild) build_tree(mut _ core.LayoutCx) core.Element {
	return r.build.run(r.i)
}

// RowFnBox carries a RowFn as bound data, for the closure-taking constructors.
struct RowFnBox {
	f RowFn = unsafe { nil }
}

fn call_row_fn(b RowFnBox, i int) core.Element {
	return b.f(i)
}

fn row_bound(f RowFn) core.Bound[int, core.Element] {
	return core.bind_fn[RowFnBox, int, core.Element](RowFnBox{f}, call_row_fn)
}

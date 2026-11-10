// VariableList: a virtualized list whose rows size to their content (specs/08). Rows are
// measured when they become visible; unmeasured rows count with the estimate. The row at
// the top keeps its bottom edge in place when its measured height differs, so scrolling up
// into unmeasured rows never moves what is on screen.
module ui

import quadra.core
import quadra.prim
import quadra.layout

@[heap]
pub struct VariableList {
pub mut:
	id       string
	count    int
	estimate f32 = 24
	reveal   int = -1
	set_y    f32 = -1
	style    layout.Style
mut:
	build  core.Bound[int, core.Element]
	node   layout.NodeId
	rows   []VisibleRow
	scroll ListScroll
	offset f32
	total  f32
}

struct Measured {
mut:
	el core.Element
	h  f32
}

// list fills its parent by default; render(i) builds row i, whose height is its content's.
pub fn list(id string, count int, render RowFn) &VariableList {
	return &VariableList{
		id:    id
		count: count
		build: row_bound(render)
		style: layout.Style{
			grow:   1
			width:  layout.pct(100)
			height: layout.pct(100)
		}
	}
}

// list_with is list with the data the row builder needs passed in, so the builder captures
// nothing (specs/14). D must not be a pointer type.
pub fn list_with[D](id string, count int, data D, render fn (d D, i int) core.Element) &VariableList {
	return &VariableList{
		id:    id
		count: count
		build: core.bind_fn[D, int, core.Element](data, render)
		style: layout.Style{
			grow:   1
			width:  layout.pct(100)
			height: layout.pct(100)
		}
	}
}

// estimate is the height assumed for rows not yet measured (24 by default).
pub fn (self_ &VariableList) estimate(h f32) &VariableList {
	mut l := unsafe { self_ }
	l.estimate = h
	return l
}

pub fn (self_ &VariableList) height(h f32) &VariableList {
	mut l := unsafe { self_ }
	l.style.height = layout.px(h)
	l.style.grow = 0
	return l
}

pub fn (self_ &VariableList) scroll_y(y f32) &VariableList {
	mut l := unsafe { self_ }
	l.set_y = y
	return l
}

pub fn (self_ &VariableList) reveal(i int) &VariableList {
	mut l := unsafe { self_ }
	l.reveal = i
	return l
}

pub fn (mut l VariableList) request_layout(mut cx core.LayoutCx) layout.NodeId {
	l.node = cx.tree.node(l.style, [])
	return l.node
}

pub fn (mut l VariableList) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter(l.id)
	defer {
		cx.leave()
	}
	mut st := cx.state(ListState{})
	mut hs := &st.heights
	if hs.h.len != l.count {
		hs.resize(l.count, l.estimate)
	}
	mut lcx := cx.layout_cx()
	mut anchor := l.set_y < 0 // scrolling keeps the top row still; a set offset is exact
	if l.set_y >= 0 {
		st.y = l.set_y
	}
	if l.reveal >= 0 && l.reveal < l.count {
		anchor = false
		top := f32(hs.prefix(l.reveal))
		if top < st.y {
			st.y = top
		} else if top + hs.height(l.reveal) > st.y + bounds.height {
			// measure up from the row, so it lands exactly at the bottom
			mut above := f32(0)
			for i := l.reveal; i >= 0 && above < bounds.height; i-- {
				_, h := l.measure(i, bounds.width, mut hs, mut lcx)
				above += h
			}
			st.y = f32(hs.prefix(l.reveal)) + hs.height(l.reveal) - bounds.height
		}
		l.reveal = -1
	}
	mut first, mut rows := 0, []Measured{}
	for _ in 0 .. 2 { // a second pass only when the measured rows end above the bottom
		st.y = prim.clamp(st.y, 0, prim.max(0, f32(hs.total()) - bounds.height))
		if l.count == 0 {
			break
		}
		first = hs.find(st.y)
		mut delta := st.y - f32(hs.prefix(first))
		rows = []Measured{}
		mut y := -delta
		for i := first; i < l.count && y < bounds.height; i++ {
			old := hs.height(i)
			el, h := l.measure(i, bounds.width, mut hs, mut lcx)
			if i == first && anchor && delta > 0 && h != old {
				delta = prim.max(0, delta + h - old) // keep its bottom edge still
				y = -delta
			}
			rows << Measured{el, h}
			y += h
		}
		st.y = f32(hs.prefix(first)) + delta
		if st.y <= prim.max(0, f32(hs.total()) - bounds.height) {
			break
		}
	}
	l.total = f32(hs.total())
	l.offset = st.y
	l.scroll = ListScroll{st, prim.max(0, l.total - bounds.height)}
	hb := cx.insert_hitbox(bounds, core.HitboxOptions{})
	cx.on_mouse(hb, .scroll, core.AnyListener{
		holder: voidptr(&l.scroll)
		call:   list_scroll
	}, false)
	cx.push_hitbox(hb)
	cx.push_clip(bounds)
	l.rows.clear()
	mut y := bounds.y + f32(hs.prefix(first)) - st.y
	for mut m in rows {
		r := prim.rect(bounds.x, y, bounds.width, m.h)
		m.el.prepaint(r, mut cx)
		l.rows << VisibleRow{m.el, r}
		y += m.h
	}
	cx.pop_clip()
	cx.pop_hitbox()
}

pub fn (mut l VariableList) paint(bounds prim.Rect, mut cx core.PaintCx) {
	cx.push_clip(bounds, 0)
	for mut r in l.rows {
		r.el.paint(r.bounds, mut cx)
	}
	cx.pop_clip()
	cx.scroll_thumb(bounds, l.total, l.offset, true, core.ThumbTarget{voidptr(l.scroll.state), set_list_scroll})
}

// measure builds row i at width w and records its height.
fn (l &VariableList) measure(i int, w f32, mut hs Heights, mut lcx core.LayoutCx) (core.Element, f32) {
	mut rb := RowBuild{l.build, i}
	mut el := core.tracked(mut rb, mut lcx) // rows die at the next prepaint
	h := lcx.tree.compute_width(el.request_layout(mut lcx), w)
	hs.set(i, h)
	return el, h
}

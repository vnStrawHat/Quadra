// Selection across rich text elements (specs/08): a SelectGroup is one selection over the
// RichEls that joined it with selectable_in, in prepaint order (a Markdown preview's
// blocks). Drag from one member into the next, double-click a word, triple-click a member,
// shift-click to extend, ctrl-a for all, ctrl-c copies the members joined by newlines.
module ui

import quadra.core
import quadra.gpu
import quadra.platform
import quadra.prim
import quadra.text as txt

// GroupState is what a group keeps between frames: the anchor and the head, each a member
// index and a byte offset in that member's text.
pub struct GroupState {
mut:
	focus    u64
	a_el     int
	a_at     int
	h_el     int
	h_at     int
	dragging bool
}

@[heap]
pub struct SelectGroup {
mut:
	st  &GroupState = unsafe { nil }
	els []&RichEl
}

// select_group makes a group; build one per tree and prepaint it before its members.
pub fn select_group() &SelectGroup {
	return &SelectGroup{}
}

// selectable_in makes this rich text a member of g.
pub fn (self_ &RichEl) selectable_in(g &SelectGroup) &RichEl {
	mut r := unsafe { self_ }
	r.group = unsafe { g }
	return r
}

// prepaint finds the group's state and takes ctrl-c and ctrl-a while it has the focus.
pub fn (mut g SelectGroup) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter('select-group')
	mut st := cx.state(GroupState{})
	if st.focus == 0 {
		mut c := core.Cx{
			app: cx.app
			win: cx.win
		}
		st.focus = c.focus_handle().id
	}
	g.st = st
	g.els.clear()
	node := cx.insert_key_node(st.focus, '', -1, bounds)
	cx.on_key(node, .key_down, '', core.AnyListener{ holder: voidptr(g), call: group_key })
	cx.leave()
}

// selected_text is the selection, the members' parts joined by newlines.
pub fn (g &SelectGroup) selected_text() string {
	mut parts := []string{}
	for k, el in g.els {
		lo, hi := g.range_in(k, el.plain)
		if hi > lo {
			parts << el.plain[lo..hi]
		}
	}
	return parts.join('\n')
}

// range_in is the part of member k, whose text is s, that is selected.
fn (g &SelectGroup) range_in(k int, s string) (int, int) {
	st := g.st
	mut ae, mut aa, mut he, mut ha := st.a_el, st.a_at, st.h_el, st.h_at
	if he < ae || (he == ae && ha < aa) {
		ae, aa, he, ha = he, ha, ae, aa
	}
	if k < ae || k > he {
		return 0, 0
	}
	mut lo, mut hi := 0, s.len
	if k == ae {
		lo = clamp_index(aa, s)
	}
	if k == he {
		hi = clamp_index(ha, s)
	}
	return lo, hi
}

// point_at is the member and offset under a window point: the member beside it on its row
// (table cells), else the start of the next member down, else the end of the last.
fn (g &SelectGroup) point_at(x f32, y f32) (int, int) {
	mut best, mut dist := -1, f32(0)
	for k, el in g.els {
		b := el.bounds
		if y < b.y || y >= b.y + b.height {
			continue
		}
		d := if x < b.x { b.x - x } else { prim.max(0, x - b.x - b.width) }
		if best < 0 || d < dist {
			best, dist = k, d
		}
	}
	if best >= 0 {
		mut el := g.els[best]
		return best, el.index_at(x, y)
	}
	for k, el in g.els {
		if el.bounds.y > y {
			return k, 0
		}
	}
	if g.els.len == 0 {
		return 0, 0
	}
	return g.els.len - 1, g.els.last().plain.len
}

// join adds r to its group and returns the hitbox that selects in it.
fn (mut r RichEl) join(bounds prim.Rect, mut cx core.PrepaintCx) int {
	mut g := r.group
	r.member = g.els.len
	r.bounds = bounds
	g.els << r
	r.offs = []int{cap: r.spans.len}
	mut at := 0
	for sp in r.spans {
		r.offs << at
		at += sp.text.len
	}
	hb := cx.insert_hitbox(bounds, core.HitboxOptions{
		cursor:   platform.Cursor.ibeam
		focus_id: g.st.focus
	})
	cx.on_mouse(hb, .mouse_down, core.AnyListener{ holder: voidptr(r), call: group_down }, false)
	cx.on_mouse(hb, .mouse_move, core.AnyListener{ holder: voidptr(r), call: group_move }, false)
	return hb
}

fn piece_start(r &RichEl, p txt.RichPiece) int {
	return r.offs[p.run] + p.start
}

fn line_end(r &RichEl, line txt.RichLine) int {
	return if line.pieces.len == 0 {
		-1
	} else {
		r.offs[line.pieces.last().run] + line.pieces.last().end
	}
}

// index_at maps a window point to a byte offset in plain.
fn (mut r RichEl) index_at(x f32, y f32) int {
	rt := r.layout_at(r.bounds.width)
	mut prev := -1 // the end of the line above
	for i, line in rt.lines {
		if y - r.bounds.y >= line.y + line.height && i < rt.lines.len - 1 {
			if line.pieces.len > 0 {
				prev = line_end(r, line)
			} else {
				prev++ // an empty line is just its newline
			}
			continue
		}
		if line.pieces.len == 0 { // an empty line: after the newline above
			return clamp_index(prev + 1, r.plain)
		}
		lx := x - r.bounds.x
		for k, p in line.pieces {
			if lx < p.x + p.width || k == line.pieces.len - 1 {
				i0 := piece_start(r, p)
				return i0 + p.layout.index_for_x(lx - p.x) // the piece's own layout
			}
		}
	}
	return 0
}

fn group_down(holder voidptr, _ voidptr, event voidptr, mut cx core.Cx) {
	mut r := unsafe { &RichEl(holder) }
	e := unsafe { &core.MouseDownEvent(event) }
	if e.button != .left || r.group.st == unsafe { nil } {
		return
	}
	mut st := r.group.st
	k := r.member
	i := r.index_at(e.x, e.y)
	if e.clicks >= 3 {
		st.a_el, st.a_at, st.h_el, st.h_at = k, 0, k, r.plain.len
	} else if e.clicks == 2 {
		a, b := word_at(r.plain, i)
		st.a_el, st.a_at, st.h_el, st.h_at = k, a, k, b
	} else {
		if !e.mods.has(.shift) {
			st.a_el, st.a_at = k, i
		}
		st.h_el, st.h_at = k, i
		st.dragging = true
		cx.capture_mouse()
	}
	cx.win.invalidate_paint()
}

fn group_move(holder voidptr, _ voidptr, event voidptr, mut cx core.Cx) {
	r := unsafe { &RichEl(holder) }
	e := unsafe { &core.MouseMoveEvent(event) }
	g := r.group
	if g.st == unsafe { nil } || !g.st.dragging {
		return
	}
	mut st := g.st
	if !e.pressed {
		st.dragging = false
		return
	}
	k, i := g.point_at(e.x, e.y)
	st.h_el, st.h_at = k, i
	cx.win.invalidate_paint()
}

fn group_key(holder voidptr, _ voidptr, event voidptr, mut cx core.Cx) {
	g := unsafe { &SelectGroup(holder) }
	e := unsafe { &core.KeyDownEvent(event) }
	if g.st == unsafe { nil } || !e.mods.has(.ctrl) {
		return
	}
	mut st := g.st
	match e.key {
		.c, .insert {
			s := g.selected_text()
			if s == '' {
				return
			}
			cx.clipboard_write(s)
		}
		.a {
			if g.els.len == 0 {
				return
			}
			st.a_el, st.a_at, st.h_el, st.h_at = 0, 0, g.els.len - 1, g.els.last().plain.len
			cx.win.invalidate_paint()
		}
		else {
			return
		}
	}

	cx.stop_propagation()
}

// paint_group_selection draws this member's part of the selection behind the glyphs, one
// band per line, while the group has the focus.
fn (r &RichEl) paint_group_selection(bounds prim.Rect, rt &txt.RichText, mut cx core.PaintCx) {
	g := r.group
	if g.st == unsafe { nil } || r.offs.len != r.spans.len || !cx.is_focused(g.st.focus) {
		return
	}
	lo, hi := g.range_in(r.member, r.plain)
	if hi <= lo {
		return
	}
	for line in rt.lines {
		mut x0, mut x1 := f32(-1), f32(-1)
		for p in line.pieces {
			s := piece_start(r, p)
			a := if lo > s { lo } else { s }
			b := if hi < s + p.end - p.start { hi } else { s + p.end - p.start }
			if b <= a {
				continue
			}
			px0 := p.x + p.layout.x_for_index(a - s)
			px1 := p.x + p.layout.x_for_index(b - s)
			if x0 < 0 || px0 < x0 {
				x0 = px0
			}
			x1 = prim.max(x1, px1)
		}
		if x0 < 0 {
			continue
		}
		if hi > line_end(r, line) && x1 - x0 < 4 {
			x1 = x0 + 4 // a selected line break shows as a sliver
		}
		cx.quad(gpu.Quad{
			bounds:     prim.rect(bounds.x + x0, bounds.y + line.y, x1 - x0, line.height)
			background: cx.win.selection_color
		})
	}
}

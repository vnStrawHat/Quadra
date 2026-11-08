// Selectable text: drag to select, double-click for a word, triple-click for everything,
// shift-click to extend, ctrl-c to copy, ctrl-a to select all. For content (logs, values,
// code, paragraphs); labels and controls stay unselectable like native UI.
// ponytail: a selection lives in one text element; selecting across elements needs a
// window-level selection model.
module ui

import quadra.core
import quadra.prim
import quadra.gpu
import quadra.platform
import quadra.text as txt

pub struct SelectState {
mut:
	focus    u64
	anchor   int
	head     int
	dragging bool
}

// selectable lets the user select and copy this text.
pub fn (self_ &TextEl) selectable() &TextEl {
	mut t := unsafe { self_ }
	t.select = true
	return t
}

fn (mut t TextEl) prepaint_select(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter('')
	mut st := cx.state(SelectState{})
	if st.focus == 0 {
		mut c := core.Cx{
			app: cx.app
			win: cx.win
		}
		st.focus = c.focus_handle().id
	}
	st.anchor, st.head = clamp_index(st.anchor, t.content), clamp_index(st.head, t.content)
	t.sel = st
	t.bounds = bounds
	hb := cx.insert_hitbox(bounds, core.HitboxOptions{
		cursor:   platform.Cursor.ibeam
		focus_id: st.focus
	})
	cx.on_mouse(hb, .mouse_down, core.AnyListener{ holder: voidptr(t), call: select_down }, false)
	cx.on_mouse(hb, .mouse_move, core.AnyListener{ holder: voidptr(t), call: select_move }, false)
	node := cx.insert_key_node(st.focus, '', -1, bounds)
	cx.on_key(node, .key_down, '', core.AnyListener{ holder: voidptr(t), call: select_key })
	cx.leave()
}

fn clamp_index(i int, s string) int {
	return if i < 0 {
		0
	} else if i > s.len {
		s.len
	} else {
		i
	}
}

// index_at maps a window point to a byte offset (a grapheme boundary) in the content.
fn (t &TextEl) index_at(x f32, y f32) int {
	mut ts := t.ts
	w := ts.wrap(t.content, t.resolved, t.bounds.width + 0.5)
	if w.lines.len == 0 {
		return 0
	}
	mut line := w.lines.last()
	for l in w.lines {
		if y - t.bounds.y < l.y + w.line_height {
			line = l
			break
		}
	}
	i := line.start + line.layout.index_for_x(x - t.bounds.x)
	return if i > line.end { line.end } else { i }
}

fn is_word_byte(c u8) bool {
	return c >= 0x80 || c.is_alnum() || c in [`_`, `-`, `.`]
}

// word_at is the run of word characters around byte offset i.
fn word_at(s string, i int) (int, int) {
	mut a, mut b := i, i
	for a > 0 && is_word_byte(s[a - 1]) {
		a--
	}
	for b < s.len && is_word_byte(s[b]) {
		b++
	}
	for a > 0 && s[a] & 0xC0 == 0x80 { // never start inside a UTF-8 sequence
		a--
	}
	return a, b
}

fn select_down(holder voidptr, _ voidptr, event voidptr, mut cx core.Cx) {
	mut t := unsafe { &TextEl(holder) }
	e := unsafe { &core.MouseDownEvent(event) }
	if e.button != .left || t.sel == unsafe { nil } {
		return
	}
	mut st := t.sel
	i := t.index_at(e.x, e.y)
	if e.clicks >= 3 {
		st.anchor, st.head = 0, t.content.len
	} else if e.clicks == 2 {
		st.anchor, st.head = word_at(t.content, i)
	} else {
		if !e.mods.has(.shift) {
			st.anchor = i
		}
		st.head = i
		st.dragging = true
		cx.capture_mouse()
	}
	cx.win.invalidate_paint()
}

fn select_move(holder voidptr, _ voidptr, event voidptr, mut cx core.Cx) {
	mut t := unsafe { &TextEl(holder) }
	e := unsafe { &core.MouseMoveEvent(event) }
	if t.sel == unsafe { nil } || !t.sel.dragging {
		return
	}
	mut st := t.sel
	if !e.pressed {
		st.dragging = false
		return
	}
	st.head = t.index_at(e.x, e.y)
	cx.win.invalidate_paint()
}

fn select_key(holder voidptr, _ voidptr, event voidptr, mut cx core.Cx) {
	mut t := unsafe { &TextEl(holder) }
	e := unsafe { &core.KeyDownEvent(event) }
	if t.sel == unsafe { nil } || !e.mods.has(.ctrl) {
		return
	}
	mut st := t.sel
	match e.key {
		.c, .insert {
			lo, hi := st.range()
			if hi <= lo {
				return
			}
			cx.clipboard_write(t.content[lo..hi])
		}
		.a {
			st.anchor, st.head = 0, t.content.len
			cx.win.invalidate_paint()
		}
		else {
			return
		}
	}

	cx.stop_propagation()
}

fn (st &SelectState) range() (int, int) {
	return if st.anchor < st.head { st.anchor, st.head } else { st.head, st.anchor }
}

// paint_selection draws the selection behind the glyphs while the text has focus.
fn (t &TextEl) paint_selection(bounds prim.Rect, w_lines []txt.WrappedLine, lh f32, mut cx core.PaintCx) {
	if t.sel == unsafe { nil } || !cx.is_focused(t.sel.focus) {
		return
	}
	lo, hi := t.sel.range()
	if hi <= lo {
		return
	}
	for line in w_lines {
		a := if lo > line.start { lo } else { line.start }
		b := if hi < line.end { hi } else { line.end }
		if a > b || (a == b && !(lo < line.start && hi > line.end)) {
			continue
		}
		x0 := line.layout.x_for_index(a - line.start)
		mut x1 := line.layout.x_for_index(b - line.start)
		if hi > line.end && x1 - x0 < 4 {
			x1 = x0 + 4 // a selected line break shows as a sliver
		}
		cx.quad(gpu.Quad{
			bounds:     prim.rect(bounds.x + x0, bounds.y + line.y, x1 - x0, lh)
			background: cx.win.selection_color
		})
	}
}

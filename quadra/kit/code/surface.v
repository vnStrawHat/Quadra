// Surface: the one element that paints an editor (specs/18, View). Paint walks the visible
// lines only; each line is one cached layout painted per token by glyph range.
module code

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.layout
import quadra.text

pub const font_size = f32(13)
pub const blink_ms = 500.0

@[heap]
pub struct Surface {
mut:
	e    &Editor = unsafe { nil }
	node layout.NodeId
}

pub fn (mut s Surface) request_layout(mut cx core.LayoutCx) layout.NodeId {
	s.node = cx.tree.node(layout.Style{
		grow:   1
		width:  layout.pct(100)
		height: layout.pct(100)
	}, [])
	return s.node
}

pub fn (mut s Surface) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	mut e := s.e
	th := kit.theme_of(cx.app)
	e.style = text.TextStyle{
		family:      th.mono_family
		size:        th.scaled(th.mono_size)
		line_height: th.mono_line
	}
	mut ts := cx.text
	m := ts.layout_line('M', e.style)
	e.cell = prim.Size{m.width, f32(math.ceil(e.style.line_px()))}
	asc, desc := ts.line_metrics(e.style)
	e.base = f32(math.round((e.cell.height - asc - desc) / 2 + asc))
	n := e.doc.buf.line_count()
	// The gutter (specs/15): at least four digits, two cells before them, three after.
	digits := if e.line_numbers { math.max(4, n.str().len) } else { 0 }
	e.gutter = if e.line_numbers { f32(digits + 5) * e.cell.width } else { 8 }
	e.bounds = prim.rect(bounds.x + e.gutter, bounds.y, bounds.width - e.gutter, bounds.height)
	e.sync_rows()
	if e.reveal {
		e.reveal_primary(ts)
		e.reveal = false
	}
	e.clamp_scroll()
	first, last := e.visible()
	for k in first .. last + 1 {
		rows := e.first_row(k + 1) - e.first_row(k)
		cx.register_text(e.doc.buf.line(k), prim.rect(e.bounds.x, e.line_y(k), e.bounds.width,
			e.cell.height * f32(rows)))
	}
}

// visible is the range of lines with a row that intersects the view.
fn (e &Editor) visible() (int, int) {
	if e.cell.height <= 0 {
		return 0, -1
	}
	first := int(e.scroll.y / e.cell.height)
	last := int((e.scroll.y + e.bounds.height) / e.cell.height)
	return e.row_line(first), e.row_line(math.min(last, e.row_count() - 1))
}

// line_y is the y of line n's first row.
fn (e &Editor) line_y(n int) f32 {
	return e.row_y(e.first_row(n))
}

fn (mut e Editor) reveal_primary(ts &text.TextSystem) {
	x, r := e.caret(ts, e.doc.primary().head)
	y := f32(r) * e.cell.height
	if y < e.scroll.y {
		e.scroll.y = y
	} else if y + e.cell.height > e.scroll.y + e.bounds.height {
		e.scroll.y = y + e.cell.height - e.bounds.height
	}
	margin := e.cell.width * 4
	if x + margin > e.widest {
		e.widest = x + margin
	}
	if x < e.scroll.x + margin {
		e.scroll.x = prim.max(0, x - margin)
	} else if x > e.scroll.x + e.bounds.width - margin {
		e.scroll.x = x - e.bounds.width + margin
	}
}

pub fn (mut s Surface) paint(bounds prim.Rect, mut cx core.PaintCx) {
	mut e := s.e
	th := kit.theme_of(cx.app)
	cx.quad(gpu.Quad{
		bounds:     bounds
		background: th.p.canvas
	})
	focused := cx.is_focused(e.focus.id)
	first, last := e.visible()
	hits := if e.find_open { e.find.matches(e.doc) } else { []int{} }
	mut hit := first_hit(hits, e.doc.buf.line_start(math.max(first, 0)))
	cx.push_clip(e.bounds, 0)
	for n in first .. last + 1 {
		hit = e.paint_line(n, hits, hit, focused, th, mut cx)
	}
	e.paint_bracket(first, last, th, mut cx)
	e.paint_cursors(focused, first, last, th, mut cx)
	cx.pop_clip()
	e.paint_gutter(bounds, first, last, focused, th, mut cx)
	// The thumb spans what clamp_scroll allows: the rows plus two of overscroll.
	cx.scroll_thumb(e.bounds, f32(e.row_count() + 2) * e.cell.height, e.scroll.y, true, core.ThumbTarget{voidptr(e), set_editor_scroll})
}

fn set_editor_scroll(holder voidptr, offset f32, _ bool) {
	mut e := unsafe { &Editor(holder) }
	e.scroll = prim.Point{e.scroll.x, offset}
	e.clamp_scroll()
}

// first_hit is the index (into start, end pairs) of the first match ending at or after off.
fn first_hit(hits []int, off int) int {
	mut lo, mut hi := 0, hits.len / 2
	for lo < hi {
		mid := (lo + hi) / 2
		if hits[2 * mid + 1] < off {
			lo = mid + 1
		} else {
			hi = mid
		}
	}
	return 2 * lo
}

fn (mut e Editor) paint_gutter(bounds prim.Rect, first int, last int, focused bool, th &kit.Theme, mut cx core.PaintCx) {
	if !e.line_numbers {
		return
	}
	ed := th.code.editor
	d := e.doc
	right := bounds.x + e.gutter - 3 * e.cell.width
	for n in first .. last + 1 {
		y := e.line_y(n)
		rows := f32(e.first_row(n + 1) - e.first_row(n))
		ls, le := d.buf.line_start(n), d.buf.line_end(n)
		active := d.cursors.any(it.head >= ls && it.head <= le)
		l := cx.text.layout_line((n + 1).str(), e.style)
		cx.glyphs(l, right - l.width, y + e.base, if active && focused {
			ed.gutter_active
		} else {
			ed.gutter
		})
		mark := e.marks[n] or { Mark.none }
		if mark != .none {
			cx.quad(gpu.Quad{
				bounds:     prim.rect(right + e.cell.width, y, 3, e.cell.height * rows)
				background: match mark {
					.add { th.p.ok }
					.change { th.p.warn }
					else { th.p.error }
				}
			})
		}
	}
}

fn (mut e Editor) paint_cursors(focused bool, first int, last int, th &kit.Theme, mut cx core.PaintCx) {
	if !focused {
		return
	}
	d := e.doc
	ed := th.code.editor
	elapsed := cx.now_ms() - e.last_edit
	phase := int(elapsed / blink_ms)
	cx.request_paint_after(int(f64(phase + 1) * blink_ms - elapsed) + 1)
	ox := e.bounds.x - e.scroll.x
	p := d.primary()
	for c in d.cursors {
		n := d.buf.line_of(c.head)
		if n < first || n > last {
			continue
		}
		cx0, r := e.caret(cx.text, c.head)
		mut x := ox + cx0
		y := e.row_y(r)
		if c.anchor == p.anchor && c.head == p.head {
			if e.preedit != '' {
				pl := cx.text.layout_line(e.preedit, e.style)
				cx.quad(gpu.Quad{
					bounds:     prim.rect(x, y, pl.width, e.cell.height)
					background: th.p.canvas
				})
				cx.glyphs(pl, x, y + e.base, th.code.syntax.variable)
				cx.quad(gpu.Quad{
					bounds:     prim.rect(x, y + e.base + 2, pl.width, 1)
					background: th.code.syntax.variable
				})
				x += pl.x_for_index(e.preedit_cursor)
			}
			mut win := cx.win
			win.pw.set_ime_rect(prim.rect(x, y, 1, e.cell.height))
		}
		if phase % 2 == 0 {
			cx.quad(gpu.Quad{
				bounds:     prim.rect(f32(math.round(x)), y, 2, e.cell.height)
				background: ed.cursor
			})
		}
	}
}

// paint_bracket outlines the bracket next to the primary cursor and its match.
fn (mut e Editor) paint_bracket(first int, last int, th &kit.Theme, mut cx core.PaintCx) {
	a, b := e.doc.bracket_pair()
	if a < 0 {
		return
	}
	ox := e.bounds.x - e.scroll.x
	for off in [a, b] {
		n := e.doc.buf.line_of(off)
		if n < first || n > last {
			continue
		}
		x0, r := e.caret(cx.text, off)
		cx.quad(gpu.Quad{
			bounds:     prim.rect(ox + x0, e.row_y(r), e.cell.width, e.cell.height)
			background: th.code.editor.bracket_match
		})
	}
}

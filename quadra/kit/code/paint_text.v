// Painting one line: the current-line band, selections, find matches, indent guides and
// the text, colored per token from one layout.
module code

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.text

// paint_line paints line n, row by row when wrapped, and returns the index of the first
// match not yet finished.
fn (mut e Editor) paint_line(n int, hits []int, hit int, focused bool, th &kit.Theme, mut cx core.PaintCx) int {
	mut d := e.doc
	ed := th.code.editor
	src := d.buf.line(n)
	dl := display_line(src, d.tab_size)
	l := cx.text.layout_mono(dl.shown, e.style, e.cell.width)
	if l.width > e.widest {
		e.widest = l.width
	}
	r0, r1 := e.first_row(n), e.first_row(n + 1)
	ls := d.buf.line_start(n)
	le := ls + src.len
	h := e.cell.height
	if focused && d.cursors.any(it.empty() && it.head >= ls && it.head <= le) {
		cx.quad(gpu.Quad{
			bounds:     prim.rect(e.bounds.x, e.row_y(r0), e.bounds.width, h * f32(r1 - r0))
			background: ed.line_highlight
		})
	}
	sel := if focused { ed.selection } else { ed.selection_inactive }
	p := d.primary()
	mut hl := e.hl
	toks := hl.tokens(mut d, n)
	plain := th.code.syntax.variable
	for r in r0 .. r1 {
		ra, rb := e.row_span(n, r)
		row := Row{
			a:    ra
			b:    math.min(rb, dl.shown.len)
			last: r == r1 - 1
			ox:   e.bounds.x - e.scroll.x - l.x_for_index(ra)
			top:  e.row_y(r)
		}
		for c in d.cursors {
			if c.empty() || c.end() <= ls || c.start() > le {
				continue
			}
			a, b := row.clip(dl.to_shown(math.max(c.start(), ls) - ls),
				dl.to_shown(math.min(c.end(), le) - ls)) or { continue }
			x0, x1 := l.x_for_index(a), l.x_for_index(b)
			tail := if c.end() > le && row.last { e.cell.width * 0.6 } else { f32(0) }
			cx.quad(gpu.Quad{
				bounds:     prim.rect(row.ox + x0, row.top, x1 - x0 + tail, h)
				background: sel
			})
		}
		// Find matches on this line (hits are sorted start, end pairs).
		mut k := hit
		for k + 1 < hits.len && hits[k] <= le {
			if hits[k + 1] >= ls {
				if a, b := row.clip(dl.to_shown(math.max(hits[k], ls) - ls), dl.to_shown(math.min(hits[
					k + 1], le) - ls))
				{
					x0, x1 := l.x_for_index(a), l.x_for_index(b)
					cur := hits[k] == p.start() && hits[k + 1] == p.end()
					cx.quad(gpu.Quad{
						bounds:     prim.rect(row.ox + x0, row.top, prim.max(x1 - x0, 2), h)
						background: if cur { ed.find_current } else { ed.find_match }
					})
				}
			}
			k += 2
		}
		if r == r0 {
			e.paint_guides(dl.shown, row.ox, row.top, ed.indent_guide, mut cx)
		}
		// Text: plain between tokens, each token in its color, all from the one layout.
		base := row.top + e.base
		mut pos := 0
		for t in toks {
			a, b := dl.to_shown(t.start), dl.to_shown(t.end)
			row.glyphs(l, base, plain, pos, a, mut cx)
			row.glyphs(l, base, syntax_color(th.code.syntax, t.kind), a, b, mut cx)
			pos = b
		}
		row.glyphs(l, base, plain, pos, dl.shown.len, mut cx)
	}
	mut next := hit
	for next + 1 < hits.len && hits[next + 1] <= le {
		next += 2
	}
	return next
}

// Row is one visual row of a line being painted: its display range and origin.
struct Row {
	a    int
	b    int
	last bool
	ox   f32 // x of the line's display offset 0 for this row
	top  f32
}

// clip is the part of the display range s0..s1 on this row (none: not on it). A range that
// starts where the next row starts belongs to the next row.
fn (r &Row) clip(s0 int, s1 int) ?(int, int) {
	if s1 < r.a || s0 > r.b || (s0 == r.b && !r.last) {
		return none
	}
	return math.max(s0, r.a), math.min(s1, r.b)
}

fn (r &Row) glyphs(l &text.LineLayout, base f32, color prim.Color, a int, b int, mut cx core.PaintCx) {
	x, y := math.max(a, r.a), math.min(b, r.b)
	if x < y {
		cx.glyphs_range(l, r.ox, base, color, x, y)
	}
}

// paint_guides draws a thin line at every indent stop inside the leading whitespace.
fn (e &Editor) paint_guides(shown string, ox f32, top f32, color prim.Color, mut cx core.PaintCx) {
	tab := e.doc.tab_size
	mut lead := 0
	for lead < shown.len && shown[lead] == ` ` {
		lead++
	}
	if lead == shown.len {
		return
	}
	for c := tab; c < lead; c += tab {
		cx.quad(gpu.Quad{
			bounds:     prim.rect(ox + f32(c) * e.cell.width, top, 1, e.cell.height)
			background: color
		})
	}
}

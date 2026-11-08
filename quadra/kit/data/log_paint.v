// LogSurface: paints the visible log lines, a timestamp gutter and ANSI colors from the
// terminal palette. Only painted lines are decoded.
module data

import math
import quadra.core
import quadra.prim
import quadra.kit
import quadra.kit.term
import quadra.layout
import quadra.text

struct LogSurface {
mut:
	lv   &LogView = unsafe { nil }
	node layout.NodeId
}

fn (mut s LogSurface) request_layout(mut cx core.LayoutCx) layout.NodeId {
	s.node = cx.tree.node(layout.Style{
		grow:   1
		width:  layout.pct(100)
		height: layout.pct(100)
	}, [])
	return s.node
}

fn (mut s LogSurface) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	mut lv := s.lv
	th := kit.theme_of(cx.app)
	lv.style = text.TextStyle{
		family:      th.mono_family
		size:        th.scaled(th.text_sm)
		line_height: 1.5
	}
	mut ts := cx.text
	m := ts.layout_line('M', lv.style)
	lv.cell = prim.Size{m.width, f32(math.ceil(lv.style.line_px()))}
	asc, desc := ts.line_metrics(lv.style)
	lv.base = f32(math.round((lv.cell.height - asc - desc) / 2 + asc))
	lv.gutter = if lv.timestamps { 12 * lv.cell.width + 20 } else { 8 }
	lv.bounds = prim.rect(bounds.x + lv.gutter, bounds.y, bounds.width - lv.gutter, bounds.height)
	max_y := lv.max_y()
	lv.scroll.y = if lv.follow { max_y } else { prim.clamp(lv.scroll.y, 0, max_y) }
	first, last := lv.visible()
	for k in first .. last + 1 {
		i := lv.shown_line(k)
		plain, _ := term.style_spans(lv.store.line(i))
		cx.register_text(plain,
			prim.rect(lv.bounds.x, lv.row_y(k), lv.bounds.width, lv.cell.height))
	}
}

fn (lv &LogView) visible() (int, int) {
	if lv.cell.height <= 0 {
		return 0, -1
	}
	first := int(lv.scroll.y / lv.cell.height)
	last := int((lv.scroll.y + lv.bounds.height) / lv.cell.height)
	return first, math.min(last, lv.shown_count() - 1)
}

fn (lv &LogView) row_y(k int) f32 {
	return lv.bounds.y + f32(k) * lv.cell.height - lv.scroll.y
}

fn (mut s LogSurface) paint(bounds prim.Rect, mut cx core.PaintCx) {
	mut lv := s.lv
	th := kit.theme_of(cx.app)
	first, last := lv.visible()
	cx.push_clip(bounds, 0)
	for k in first .. last + 1 {
		lv.paint_row(k, bounds, th, mut cx)
	}
	cx.pop_clip()
}

fn (mut lv LogView) paint_row(k int, bounds prim.Rect, th &kit.Theme, mut cx core.PaintCx) {
	i := lv.shown_line(k)
	y := lv.row_y(k) + lv.base
	raw := lv.store.line(i)
	faint := lv.mode == .dim && lv.matcher.active && !lv.matcher.matches(raw)
	alpha := if faint { f32(0.4) } else { f32(1) }
	if lv.timestamps && lv.store.at(i) != 0 {
		tl := cx.text.layout_line(clock(lv.store.at(i)), lv.style)
		cx.glyphs(tl, bounds.x + 8, y, th.p.text_3.alpha(th.p.text_3.a * alpha))
	}
	plain, spans := term.style_spans(raw)
	l := cx.text.layout_line(plain, lv.style)
	if l.width > lv.widest {
		lv.widest = l.width
	}
	cx.push_clip(lv.bounds, 0)
	x := lv.bounds.x - lv.scroll.x
	fg := th.p.text.alpha(th.p.text.a * alpha)
	pal := th.code.terminal
	mut pos := 0
	for sp in spans {
		if sp.start > pos {
			cx.glyphs_range(l, x, y, fg, pos, sp.start)
		}
		c := term.resolve(sp.pen.fg, th.p.text, pal)
		cx.glyphs_range(l, x, y, c.alpha(c.a * alpha * if sp.pen.flags & term.f_faint != 0 {
			f32(0.6)
		} else {
			f32(1)
		}), sp.start, sp.end)
		pos = sp.end
	}
	if pos < plain.len {
		cx.glyphs_range(l, x, y, fg, pos, plain.len)
	}
	cx.pop_clip()
}

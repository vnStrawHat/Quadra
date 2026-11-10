// Field: paints an input's text, selection, caret and IME preedit, and maps points to
// text offsets. Single-line fields scroll horizontally to keep the caret visible.
module input

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.layout
import quadra.text

const caret_blink_ms = 530.0

@[heap]
pub struct Field {
mut:
	v    &TextInput = unsafe { nil }
	node layout.NodeId
	ts   &text.TextSystem = unsafe { nil }
}

fn measure_field(ctx voidptr, kw f32, _ f32, aw f32, _ f32) prim.Size {
	mut f := unsafe { &Field(ctx) }
	v := f.v
	lh := v.style.line_px()
	if !v.multiline {
		return prim.size(if kw != layout.unknown { kw } else { 120 }, lh)
	}
	width := if kw != layout.unknown {
		kw
	} else if aw < layout.unbounded {
		aw
	} else {
		f32(300)
	}
	w := f.ts.wrap(v.shown() + ' ', v.style, width)
	mut rows := prim.max(f32(v.rows), f32(w.lines.len))
	if v.max_rows > 0 {
		rows = prim.min(rows, f32(v.max_rows))
	}
	return prim.size(width, rows * lh)
}

pub fn (mut f Field) request_layout(mut cx core.LayoutCx) layout.NodeId {
	f.ts = cx.text
	f.node = cx.tree.leaf(layout.Style{
		grow:  1
		width: layout.pct(100)
	}, measure_field, voidptr(f))
	return f.node
}

pub fn (mut f Field) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	mut v := f.v
	v.field_bounds = bounds
	cx.register_text(v.shown(), bounds)
	if !v.multiline {
		// Keep the caret inside the visible width.
		l := cx.text.layout_line(v.shown(), v.style)
		x := l.x_for_index(v.display_index(v.buf.head))
		if x - v.scroll_x > bounds.width - 2 {
			v.scroll_x = x - bounds.width + 2
		}
		if x - v.scroll_x < 0 {
			v.scroll_x = x
		}
		v.scroll_x = prim.clamp(v.scroll_x, 0, prim.max(0, l.width - bounds.width + 2))
		if v.center && l.width < bounds.width {
			v.scroll_x = -(bounds.width - l.width) / 2 // a negative scroll shifts the text right
		}
	}
}

// Line is one visual line: its byte range in the shown text and its layout.
struct Line {
	start  int
	layout &text.LineLayout
	y      f32
}

fn (f &Field) lines(ts &text.TextSystem, width f32) []Line {
	mut sys := unsafe { ts }
	v := f.v
	shown := v.shown()
	if !v.multiline {
		return [Line{0, sys.layout_line(shown, v.style), 0}]
	}
	w := sys.wrap(shown, v.style, width)
	return w.lines.map(Line{it.start, it.layout, it.y})
}

pub fn (mut f Field) paint(bounds prim.Rect, mut cx core.PaintCx) {
	v := f.v
	focused := cx.is_focused(v.focus.id)
	asc, desc := cx.text.line_metrics(v.style)
	lh := v.style.line_px()
	lead := (lh - asc - desc) / 2
	ox := bounds.x - if v.multiline { f32(0) } else { v.scroll_x }
	cx.push_clip(bounds, 0)
	defer {
		cx.pop_clip()
	}
	if v.buf.text == '' && v.preedit == '' {
		l := cx.text.layout_line(v.placeholder, v.style)
		cx.glyphs(l, bounds.x, bounds.y + lead + asc, v.placeholder_color)
	}
	lines := f.lines(cx.text, bounds.width)
	s, e := v.buf.selection()
	ds, de := v.display_index(s), v.display_index(e)
	for ln in lines {
		top := bounds.y + ln.y
		line_end := ln.start + ln.layout.text.len
		if focused && ds < de && de >= ln.start && ds <= line_end {
			x0 := ln.layout.x_for_index(math.max(ds, ln.start) - ln.start)
			x1 := ln.layout.x_for_index(math.min(de, line_end) - ln.start)
			cx.quad(gpu.Quad{
				bounds:     prim.rect(ox + x0, top, prim.max(x1 - x0, 2), lh)
				background: v.selection_color
			})
		}
		cx.glyphs(ln.layout, ox, top + lead + asc, v.style.color)
	}
	if !focused {
		return
	}
	// Caret (and IME preedit, drawn underlined at the caret without touching the buffer).
	caret := v.display_index(v.buf.head)
	mut cx_ := ox
	mut cy := bounds.y
	for ln in lines {
		if caret >= ln.start && caret <= ln.start + ln.layout.text.len {
			cx_ = ox + ln.layout.x_for_index(caret - ln.start)
			cy = bounds.y + ln.y
		}
	}
	if v.preedit != '' {
		pl := cx.text.layout_line(v.preedit, v.style)
		cx.quad(gpu.Quad{
			bounds:     prim.rect(cx_, cy, pl.width, lh)
			background: v.style.color.alpha(0.06)
		})
		cx.glyphs(pl, cx_, cy + lead + asc, v.style.color)
		cx.line(gpu.LinePrim{
			bounds:    prim.rect(cx_, cy + lead + asc + 2, pl.width, 1)
			color:     v.style.color
			thickness: 1
		})
		cx_ += pl.x_for_index(v.preedit_cursor)
	}
	mut win := cx.win
	win.pw.set_ime_rect(prim.rect(cx_, cy, 1, lh))
	elapsed := cx.now_ms() - v.last_edit
	phase := int(elapsed / caret_blink_ms)
	if phase % 2 == 0 {
		cx.quad(gpu.Quad{
			bounds:     prim.rect(f32(math.round(cx_)), cy + lead * 0.5, 1.5, lh - lead)
			background: v.caret_color
		})
	}
	cx.request_paint_after(int(f64(phase + 1) * caret_blink_ms - elapsed) + 1)
}

// index_at maps a window point to a text offset (mouse selection).
pub fn (v &TextInput) index_at(ts &text.TextSystem, p prim.Point) int {
	b := v.field_bounds
	f := &Field{
		v: unsafe { v }
	}
	lines := f.lines(ts, b.width)
	mut best := lines[0]
	for ln in lines {
		if p.y >= b.y + ln.y {
			best = ln
		}
	}
	ox := b.x - if v.multiline { f32(0) } else { v.scroll_x }
	return v.buffer_index(best.start + best.layout.index_for_x(p.x - ox))
}

// vertical moves the caret one visual line up or down, keeping its x.
pub fn (v &TextInput) vertical(ts &text.TextSystem, dir int) int {
	f := &Field{
		v: unsafe { v }
	}
	lines := f.lines(ts, v.field_bounds.width)
	caret := v.display_index(v.buf.head)
	for i, ln in lines {
		if caret >= ln.start && caret <= ln.start + ln.layout.text.len {
			j := i + dir
			if j < 0 {
				return 0
			}
			if j >= lines.len {
				return v.buf.text.len
			}
			x := ln.layout.x_for_index(caret - ln.start)
			return v.buffer_index(lines[j].start + lines[j].layout.index_for_x(x))
		}
	}
	return v.buf.head
}

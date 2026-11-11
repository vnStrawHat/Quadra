// Grid: the one element that paints a terminal (specs/17, View). Backgrounds are merged
// into one quad per run, text is one line layout per run of equal style placed on the cell
// grid, and every non-ASCII character is its own run so fallback fonts never shift columns.
module term

import math
import time
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.layout
import quadra.text

const xterm_levels = [u32(0), 95, 135, 175, 215, 255]!

pub const font_size = f32(13)
pub const blink_ms = 500

pub struct Grid {
mut:
	t    &Terminal = unsafe { nil }
	node layout.NodeId
	hbs  []int // the hovered link's hitboxes (cursor.v)
}

pub fn (mut g Grid) request_layout(mut cx core.LayoutCx) layout.NodeId {
	g.node = cx.tree.node(layout.Style{
		grow:   1
		width:  layout.pct(100)
		height: layout.pct(100)
	}, [])
	return g.node
}

// prepaint measures the cell and fits the screen (and the PTY) to the bounds.
pub fn (mut g Grid) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	mut t := g.t
	th := kit.theme_of(cx.app)
	st := t.text_style(th)
	mut ts := cx.text
	if t.b.features.len > 0 || !t.b.ligatures || t.feature_sig != '' {
		t.apply_features(st.family, mut ts)
	}
	m := ts.layout_line('0', st)
	mut scale := f32(cx.win.scale())
	if scale < 1 {
		scale = 1
	}
	// Whole device pixels: abutting cells share their edges and strokes meet.
	raw_w := if t.b.cell_width > 0 { t.b.cell_width } else { m.width }
	asc, desc := ts.line_metrics(st)
	raw_h := if st.line_px() > asc + desc { st.line_px() } else { asc + desc }
	dw := int_max(int(math.round(raw_w * scale)), 1)
	dh := int_max(int(math.round(raw_h * scale)), 1)
	t.cell = prim.Size{f32(dw) / scale, f32(dh) / scale}
	t.cell_px = [dw, dh]!
	t.screen.cell_px(dw, dh)
	e := t.b.padding
	x0 := f32(math.round((bounds.x + e.left) * scale)) / scale
	y0 := f32(math.round((bounds.y + e.top) * scale)) / scale
	t.bounds = prim.rect(x0, y0, bounds.width - e.left - e.right, bounds.height - e.top - e.bottom)
	cols := clamp(int(t.bounds.width / t.cell.width), 0, max_cols)
	rows := clamp(int(t.bounds.height / t.cell.height), 0, max_rows)
	if cols > 0 && rows > 0 && (cols != t.screen.cols || rows != t.screen.rows) {
		t.screen.resize(cols, rows)
		t.offset = clamp(t.offset, 0, t.screen.scrollback.len())
		if mut p := t.pty {
			p.resize(cols, rows) or {}
		}
	}
	g.link_hitboxes(mut cx)
}

pub fn (mut g Grid) paint(bounds prim.Rect, mut cx core.PaintCx) {
	sw := time.new_stopwatch()
	mut t := g.t
	th := kit.theme_of(cx.app)
	pal_v := t.screen.paint_palette(th.code.terminal)
	pal := &pal_v
	t.sweep_gfx(mut cx)
	if t.hooks.rows != unsafe { nil } {
		t.hooks.rows(t.hooks.holder, mut t, t.top_line())
	}
	cx.quad(gpu.Quad{
		bounds:     bounds
		background: pal.background
	})
	cx.push_clip(bounds, 0)
	st := t.text_style(th)
	inner := t.bounds
	asc, desc := cx.text.line_metrics(st)
	pad := (t.cell.height - asc - desc) / 2
	top := t.top_line()
	a, b := t.screen.sel.range()
	mut next_hit := 0
	lit := g.link_lit(mut cx)
	for next_hit < t.find.hits.len && t.find.hits[next_hit].line < top {
		next_hit++
	}
	for r in 0 .. t.screen.rows {
		line := top + r
		row := t.row_at(line)
		y := inner.y + f32(r) * t.cell.height
		cls := if r < t.classes.len { t.classes[r] } else { []u8{} }
		g.paint_backgrounds(row, y, pal, cls, mut cx)
		if row.extra != unsafe { nil } && row.extra.images.len > 0 {
			g.paint_images(row, y, mut cx)
		}
		next_hit = g.paint_hits(line, y, next_hit, th.code.editor, mut cx)
		if t.screen.sel.active && line >= a.line && line <= b.line {
			c0, c1 := t.screen.sel.cols_on(line, t.screen.cols)
			if c1 > c0 {
				cx.quad(gpu.Quad{
					bounds:     prim.rect(inner.x + f32(c0) * t.cell.width, y,
						f32(c1 - c0) * t.cell.width, t.cell.height)
					background: pal.selection
				})
			}
		}
		g.paint_text(row, y, y + pad + asc, st, pal, cls, mut cx)
		g.flush_shapes(y, mut cx)
		if lit && line >= t.hover.line && line <= t.hover.end_line {
			g.paint_link(line, y + pad + asc, pal, mut cx)
		}
	}
	g.paint_cursor(inner, st, pad + asc, pal, mut cx)
	cx.pop_clip()
	h := t.cell.height
	t.stats.paint_us += sw.elapsed().microseconds()
	t.stats.frames++
	if t.screen.alt_active {
		return
	}
	cx.scroll_thumb_with(bounds, f32(t.screen.scrollback.len() + t.screen.rows) * h, f32(top) * h,
		true, core.ThumbTarget{prim.hold(t), set_term_scroll}, kit.term_thumb(th))
}

// set_term_scroll turns a thumb offset (px from the top of the history) into rows back
// from the live screen.
fn set_term_scroll(holder prim.Holder, offset f32, _ bool) {
	mut t := prim.held[Terminal](holder)
	n := t.screen.scrollback.len()
	t.offset = clamp(n - int(offset / t.cell.height + 0.5), 0, n)
}

fn (s &Screen) colors(c Cell, pal &kit.TerminalColors) (prim.Color, prim.Color) {
	mut fg := s.color_of(c.fg, pal.foreground, pal)
	mut bg := s.color_of(c.bg, pal.background, pal)
	if c.flags & f_inverse != 0 {
		fg, bg = bg, fg
	}
	if c.flags & f_faint != 0 {
		fg = fg.alpha(fg.a * 0.6)
	}
	return fg, bg
}

// resolve maps a cell color: the theme's 16, the 6x6x6 cube and the gray ramp, or 24-bit.
pub fn resolve(c u32, def prim.Color, pal kit.TerminalColors) prim.Color {
	if c == 0 {
		return def
	}
	if c & true_color != 0 {
		return prim.rgb(c & 0xffffff)
	}
	i := int(c - 1)
	if i < 16 {
		return pal.ansi[i]
	}
	return cube_color(i)
}

// cube_color is palette 16..255: the 6x6x6 cube and the gray ramp.
fn cube_color(i int) prim.Color {
	if i < 232 {
		k := i - 16
		return prim.rgb(xterm_levels[k / 36] << 16 | xterm_levels[(k / 6) % 6] << 8 | xterm_levels[k % 6])
	}
	v := u32(8 + (i - 232) * 10)
	return prim.rgb(v << 16 | v << 8 | v)
}

fn (g &Grid) cell_bg(row Row, x int, pal &kit.TerminalColors, cls []u8) prim.Color {
	c := row.cells[x]
	s := g.t.screen
	bg := if c.flags & f_inverse != 0 { // as colors(), without the foreground
		s.color_of(c.fg, pal.foreground, pal)
	} else {
		s.color_of(c.bg, pal.background, pal)
	}
	if x < cls.len && cls[x] != 0 {
		tint := g.t.tints[cls[x]]
		if tint.has_bg {
			return tint.bg
		}
	}
	return bg
}

fn (g &Grid) paint_backgrounds(row Row, y f32, pal &kit.TerminalColors, cls []u8, mut cx core.PaintCx) {
	t := g.t
	mut x := 0
	mut bg := if row.cells.len > 0 { g.cell_bg(row, 0, pal, cls) } else { pal.background }
	for x < row.cells.len {
		mut e := x + 1
		mut next := bg
		for e < row.cells.len {
			next = g.cell_bg(row, e, pal, cls)
			if next != bg {
				break
			}
			e++
		}
		if bg != pal.background {
			cx.quad(gpu.Quad{
				bounds:     prim.rect(t.bounds.x + f32(x) * t.cell.width, y,
					f32(e - x) * t.cell.width, t.cell.height)
				background: bg
			})
		}
		x, bg = e, next
	}
}

fn (g &Grid) paint_text(row Row, y f32, baseline f32, st text.TextStyle, pal &kit.TerminalColors, cls []u8, mut cx core.PaintCx) {
	mut t := unsafe { g.t }
	cells := row.cells
	mut x := 0
	t.last_shape = -2
	for x < cells.len {
		c := cells[x]
		if c.ch == 0 || c.ch == 32 || c.ch == 9 || c.flags & (f_spacer | f_hidden) != 0 {
			g.decorate(c, x, 1, y, baseline, pal, mut cx)
			x++
			continue
		}
		if c.ch >= 0x2500 {
			idx := shape_index(c.ch)
			if idx >= 0 {
				tint := t.tint_of(cls, x)
				base, bg := t.screen.colors(c, pal)
				fg := t.text_contrast(c, t.tinted(c, tint, base), bg)
				bold := c.flags & f_bold != 0 || tint.bold
				g.push_shape(c.ch, idx, x, fg, bold)
				g.decorate(c, x, 1, y, baseline, pal, mut cx)
				x++
				continue
			}
		}
		mut e := x + 1
		mut s := rune(c.ch).str()
		if c.ch < 128 && row.marks.len == 0 {
			for e < cells.len && cells[e].ch > 32 && cells[e].ch < 128 && cells[e].same_style(c)
				&& cells[e].flags & f_hidden == 0 && (x >= cls.len || cls[e] == cls[x]) {
				s += rune(cells[e].ch).str()
				e++
			}
		} else {
			for m in row.marks {
				if m.col == x {
					s += m.s
				}
			}
			if c.flags & f_wide != 0 {
				e++
			}
		}
		tint := t.tint_of(cls, x)
		base, bg := t.screen.colors(c, pal)
		fg := t.text_contrast(c, t.tinted(c, tint, base), bg)
		cell_st := text.TextStyle{
			...st
			weight: if c.flags & f_bold != 0 || tint.bold { 700 } else { st.weight }
			italic: c.flags & f_italic != 0 || tint.italic
		}
		l := if c.ch >= 128 && t.b.fallbacks.len > 0 {
			t.fallback_layout(s, cell_st, mut cx)
		} else {
			cx.text.layout_line(s, cell_st)
		}
		if c.ch < 128 {
			cx.glyphs_cells(l, t.bounds.x + f32(x) * t.cell.width, baseline, fg, t.cell.width)
		} else {
			cx.glyphs(l, t.bounds.x + f32(x) * t.cell.width, baseline, fg)
		}
		g.decorate(c, x, e - x, y, baseline, pal, mut cx)
		if tint.underline && c.flags & f_underline == 0 {
			g.hline(t.bounds.x + f32(x) * t.cell.width, baseline + 2, f32(e - x) * t.cell.width,
				fg, mut cx)
		}
		x = e
	}
}

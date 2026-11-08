// Grid: the one element that paints a terminal (specs/17, View). Backgrounds are merged
// into one quad per run, text is one line layout per run of equal style placed on the cell
// grid, and every non-ASCII character is its own run so fallback fonts never shift columns.
module term

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.layout
import quadra.text

const xterm_levels = [u32(0), 95, 135, 175, 215, 255]!

pub const font_size = f32(13)
pub const blink_ms = 530

pub struct Grid {
mut:
	t    &Terminal = unsafe { nil }
	node layout.NodeId
}

pub fn (mut g Grid) request_layout(mut cx core.LayoutCx) layout.NodeId {
	g.node = cx.tree.node(layout.Style{
		grow:   1
		width:  layout.pct(100)
		height: layout.pct(100)
	}, [])
	return g.node
}

fn base_style(th &kit.Theme) text.TextStyle {
	return text.TextStyle{
		family:      th.mono_family
		size:        th.scaled(font_size)
		line_height: 1.25
	}
}

// prepaint measures the cell and fits the screen (and the PTY) to the bounds.
pub fn (mut g Grid) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	mut t := g.t
	th := kit.theme_of(cx.app)
	st := base_style(th)
	mut ts := cx.text
	m := ts.layout_line('M', st)
	t.cell = prim.Size{m.width, f32(math.ceil(st.line_px()))}
	t.bounds = bounds
	cols := int(bounds.width / t.cell.width)
	rows := int(bounds.height / t.cell.height)
	if cols > 0 && rows > 0 && (cols != t.screen.cols || rows != t.screen.rows) {
		t.screen.resize(cols, rows)
		t.offset = clamp(t.offset, 0, t.screen.scrollback.len())
		if mut p := t.pty {
			p.resize(cols, rows) or {}
		}
	}
}

pub fn (mut g Grid) paint(bounds prim.Rect, mut cx core.PaintCx) {
	t := g.t
	th := kit.theme_of(cx.app)
	pal := th.code.terminal
	cx.quad(gpu.Quad{
		bounds:     bounds
		background: pal.background
	})
	cx.push_clip(bounds, 0)
	st := base_style(th)
	asc, desc := cx.text.line_metrics(st)
	pad := (t.cell.height - asc - desc) / 2
	top := t.top_line()
	a, b := t.sel.range()
	for r in 0 .. t.screen.rows {
		line := top + r
		row := t.row_at(line)
		y := bounds.y + f32(r) * t.cell.height
		g.paint_backgrounds(row, y, pal, mut cx)
		if t.sel.active && line >= a.line && line <= b.line {
			c0 := if line == a.line { a.col } else { 0 }
			c1 := if line == b.line { b.col } else { t.screen.cols }
			if c1 > c0 {
				cx.quad(gpu.Quad{
					bounds:     prim.rect(bounds.x + f32(c0) * t.cell.width, y,
						f32(c1 - c0) * t.cell.width, t.cell.height)
					background: pal.selection
				})
			}
		}
		g.paint_text(row, y, y + pad + asc, st, pal, mut cx)
	}
	g.paint_cursor(bounds, st, pad + asc, pal, mut cx)
	cx.pop_clip()
	h := t.cell.height
	cx.scroll_thumb(bounds, f32(t.screen.scrollback.len() + t.screen.rows) * h, f32(top) * h, true, core.ThumbTarget{voidptr(t), set_term_scroll})
}

// set_term_scroll turns a thumb offset (px from the top of the history) into rows back
// from the live screen.
fn set_term_scroll(holder voidptr, offset f32, _ bool) {
	mut t := unsafe { &Terminal(holder) }
	n := t.screen.scrollback.len()
	t.offset = clamp(n - int(offset / t.cell.height + 0.5), 0, n)
}

fn colors(c Cell, pal kit.TerminalColors) (prim.Color, prim.Color) {
	mut fg := resolve(c.fg, pal.foreground, pal)
	mut bg := resolve(c.bg, pal.background, pal)
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
	if i < 232 {
		k := i - 16
		return prim.rgb(xterm_levels[k / 36] << 16 | xterm_levels[(k / 6) % 6] << 8 | xterm_levels[k % 6])
	}
	v := u32(8 + (i - 232) * 10)
	return prim.rgb(v << 16 | v << 8 | v)
}

fn (g &Grid) paint_backgrounds(row Row, y f32, pal kit.TerminalColors, mut cx core.PaintCx) {
	t := g.t
	mut x := 0
	for x < row.cells.len {
		_, bg := colors(row.cells[x], pal)
		mut e := x + 1
		for e < row.cells.len {
			_, b2 := colors(row.cells[e], pal)
			if b2 != bg {
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
		x = e
	}
}

fn (g &Grid) paint_text(row Row, y f32, baseline f32, st text.TextStyle, pal kit.TerminalColors, mut cx core.PaintCx) {
	t := g.t
	cells := row.cells
	mut x := 0
	for x < cells.len {
		c := cells[x]
		if c.ch == 0 || c.ch == 32 || c.flags & (f_spacer | f_hidden) != 0 {
			g.decorate(c, x, 1, y, baseline, pal, mut cx)
			x++
			continue
		}
		mut e := x + 1
		mut s := rune(c.ch).str()
		if c.ch < 128 && row.marks.len == 0 {
			for e < cells.len && cells[e].ch > 32 && cells[e].ch < 128 && cells[e].same_style(c)
				&& cells[e].flags & f_hidden == 0 {
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
		fg, _ := colors(c, pal)
		l := cx.text.layout_line(s, text.TextStyle{
			...st
			weight: if c.flags & f_bold != 0 { 700 } else { 400 }
			italic: c.flags & f_italic != 0
		})
		cx.glyphs(l, t.bounds.x + f32(x) * t.cell.width, baseline, fg)
		g.decorate(c, x, e - x, y, baseline, pal, mut cx)
		x = e
	}
}

// decorate draws underlines and strike-through across n cells.
fn (g &Grid) decorate(c Cell, x int, n int, y f32, baseline f32, pal kit.TerminalColors, mut cx core.PaintCx) {
	if c.flags & (f_underline | f_strike) == 0 {
		return
	}
	t := g.t
	fg, _ := colors(c, pal)
	x0, w := t.bounds.x + f32(x) * t.cell.width, f32(n) * t.cell.width
	if c.flags & f_underline != 0 {
		cx.quad(gpu.Quad{
			bounds:     prim.rect(x0, baseline + 2, w, 1)
			background: fg
		})
		if c.flags & f_double != 0 {
			cx.quad(gpu.Quad{
				bounds:     prim.rect(x0, baseline + 4, w, 1)
				background: fg
			})
		}
	}
	if c.flags & f_strike != 0 {
		cx.quad(gpu.Quad{
			bounds:     prim.rect(x0, y + t.cell.height * 0.5, w, 1)
			background: fg
		})
	}
}

fn (g &Grid) paint_cursor(bounds prim.Rect, st text.TextStyle, base f32, pal kit.TerminalColors, mut cx core.PaintCx) {
	t := g.t
	s := t.screen
	row := s.y + t.offset
	if !s.cursor_on || row >= s.rows || t.exited != none {
		return
	}
	focused := cx.is_focused(t.focus.id)
	if focused {
		phase := int(cx.now_ms() / blink_ms)
		cx.request_paint_after(int(f64(phase + 1) * blink_ms - cx.now_ms()) + 1)
		if phase % 2 == 1 {
			return
		}
	}
	c := s.lines[s.y].cells[s.x]
	w := if c.flags & f_wide != 0 { t.cell.width * 2 } else { t.cell.width }
	r := prim.rect(bounds.x + f32(s.x) * t.cell.width, bounds.y + f32(row) * t.cell.height, w,
		t.cell.height)
	if !focused {
		cx.quad(gpu.Quad{
			bounds:        r
			border_widths: prim.edges(1)
			border_color:  pal.cursor
		})
		return
	}
	match s.shape {
		.bar {
			cx.quad(gpu.Quad{ bounds: prim.rect(r.x, r.y, 2, r.height), background: pal.cursor })
		}
		.underline {
			cx.quad(gpu.Quad{
				bounds:     prim.rect(r.x, r.y + r.height - 2, r.width, 2)
				background: pal.cursor
			})
		}
		.block {
			cx.quad(gpu.Quad{ bounds: r, background: pal.cursor })
			if c.ch > 32 {
				l := cx.text.layout_line(rune(c.ch).str(), st)
				cx.glyphs(l, r.x, r.y + base, pal.background)
			}
		}
	}
}

// LineChart: up to 8 series over a shared time axis, a y axis with 4 grid lines, and a
// hover crosshair with the values at that point. Hover needs no state: a hitbox listener
// asks for a repaint and paint reads the mouse position.
module viz

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.layout
import quadra.text

pub struct Series {
pub:
	name   string
	values []f64
	color  ?prim.Color
}

pub struct LineChart {
pub mut:
	id       string
	series   []Series
	start_ms i64 // time of the first sample
	step_ms  i64 = 1000
	unit     string // after values in the tooltip
	height   f32 = 200
mut:
	node   layout.NodeId
	hitbox int = -1
}

pub fn line_chart(id string, series []Series) &LineChart {
	return &LineChart{
		id:     id
		series: series
	}
}

pub fn (self_ &LineChart) time(start_ms i64, step_ms i64) &LineChart {
	mut c := unsafe { self_ }
	c.start_ms, c.step_ms = start_ms, step_ms
	return c
}

pub fn (self_ &LineChart) unit(u string) &LineChart {
	mut c := unsafe { self_ }
	c.unit = u
	return c
}

pub fn (self_ &LineChart) height(h f32) &LineChart {
	mut c := unsafe { self_ }
	c.height = h
	return c
}

pub fn (mut c LineChart) request_layout(mut cx core.LayoutCx) layout.NodeId {
	c.node = cx.tree.node(layout.Style{
		width:  layout.pct(100)
		height: layout.px(c.height)
	}, [])
	return c.node
}

fn repaint_on_move(_ prim.Holder, _ prim.Holder, _ prim.Holder, mut cx core.Cx) {
	cx.win.invalidate_paint()
}

pub fn (mut c LineChart) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter(c.id)
	defer {
		cx.leave()
	}
	c.hitbox = cx.insert_hitbox(bounds, core.HitboxOptions{
		hover_sensitive: true
	})
	cx.on_mouse(c.hitbox, .mouse_move, core.AnyListener{
		call: repaint_on_move
	}, false)
	cx.register_test_id(c.id, bounds)
}

const axis_w = f32(44)
const axis_h = f32(20)

fn (c &LineChart) color_of(i int, th &kit.Theme) prim.Color {
	return c.series[i].color or { series_color(th, i) }
}

pub fn (mut c LineChart) paint(bounds prim.Rect, mut cx core.PaintCx) {
	th := kit.theme_of(cx.app)
	style := mono(th, th.text_xs)
	plot := prim.rect(bounds.x + axis_w, bounds.y + 6, bounds.width - axis_w - 8, bounds.height -
		axis_h - 6)
	max := nice_max(max_of(c.series.map(it.values)))
	// Grid lines and y labels.
	for k in 0 .. 5 {
		y := plot.y + plot.height * f32(k) / 4
		cx.quad(gpu.Quad{
			bounds:     prim.rect(plot.x, f32(math.round(y)), plot.width, 1)
			background: if k == 4 { th.p.line_strong } else { th.p.line }
		})
		l := cx.text.layout_line(short(max * f64(4 - k) / 4), style)
		cx.glyphs(l, plot.x - 6 - l.width, y + 4, th.p.text_3)
	}
	// Time labels: first, middle, last.
	n := if c.series.len > 0 { c.series[0].values.len } else { 0 }
	if n > 1 && c.start_ms > 0 {
		for k in 0 .. 3 {
			idx := (n - 1) * k / 2
			l := cx.text.layout_line(hh_mm(c.start_ms + i64(idx) * c.step_ms), style)
			x := plot.x + plot.width * f32(k) / 2 - l.width * f32(k) / 2
			cx.glyphs(l, x, plot.bottom() + 15, th.p.text_3)
		}
	}
	for i, s in c.series {
		paint_line(s.values, plot, max, c.color_of(i, th), if c.series.len == 1 {
			f32(0.12)
		} else {
			f32(0)
		}, 1.5, mut cx)
	}
	if c.hitbox >= 0 && cx.is_hovered(c.hitbox) && n > 0 {
		c.paint_hover(plot, n, th, mut cx)
	}
}

// paint_hover draws the crosshair at the mouse and a box with the values there.
fn (c &LineChart) paint_hover(plot prim.Rect, n int, th &kit.Theme, mut cx core.PaintCx) {
	mx := cx.win.mouse.x
	if mx < plot.x || mx > plot.right() {
		return
	}
	t := f64(mx - plot.x) / f64(math.max(plot.width, 1))
	idx := int(math.round(t * f64(n - 1)))
	x := plot.x + plot.width * f32(idx) / f32(math.max(n - 1, 1))
	cx.quad(gpu.Quad{
		bounds:     prim.rect(f32(math.round(x)), plot.y, 1, plot.height)
		background: th.p.text_3
	})
	style := mono(th, th.text_xs)
	mut lines := []string{}
	if c.start_ms > 0 {
		lines << hh_mm(c.start_ms + i64(idx) * c.step_ms)
	}
	for s in c.series {
		v := if idx < s.values.len { s.values[idx] } else { 0 }
		lines << '${s.name} ${short(v)}${c.unit}'
	}
	lh := f32(16)
	mut w := f32(0)
	for s in lines {
		w = prim.max(w, cx.text.layout_line(s, style).width)
	}
	w += 30
	bx := if x + 12 + w > plot.right() { x - 12 - w } else { x + 12 }
	cx.shadow(gpu.Shadow{
		bounds: prim.rect(bx, plot.y + 4, w, lh * f32(lines.len) + 10)
		radii:  prim.corners(th.radius_md)
		blur:   8
		color:  prim.rgba(0x00000040)
	})
	cx.quad(gpu.Quad{
		bounds:        prim.rect(bx, plot.y + 4, w, lh * f32(lines.len) + 10)
		background:    th.p.raised
		border_widths: prim.edges(1)
		border_color:  th.p.line
		radii:         prim.corners(th.radius_md)
	})
	for k, s in lines {
		y := plot.y + 9 + lh * f32(k) + 11
		if k > 0 || c.start_ms == 0 {
			si := if c.start_ms > 0 { k - 1 } else { k }
			cx.quad(gpu.Quad{
				bounds:     prim.rect(bx + 8, y - 7, 6, 6)
				background: c.color_of(si, th)
				radii:      prim.corners(1)
			})
		}
		l := cx.text.layout_line(s, style)
		cx.glyphs(l, bx + 18, y, th.p.text)
	}
}

fn mono(th &kit.Theme, size f32) text.TextStyle {
	return text.TextStyle{
		family: th.mono_family
		size:   th.scaled(size)
	}
}

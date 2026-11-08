// BarChart: labeled horizontal bars with their values, scaled to a rounded maximum.
module viz

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.layout

pub struct Bar {
pub:
	label string
	value f64
	color ?prim.Color
}

pub struct BarChart {
pub mut:
	id      string
	bars    []Bar
	unit    string
	label_w f32 = 160
	row_h   f32 = 24
mut:
	node layout.NodeId
}

pub fn bar_chart(id string, bars []Bar) &BarChart {
	return &BarChart{
		id:   id
		bars: bars
	}
}

pub fn (self_ &BarChart) unit(u string) &BarChart {
	mut c := unsafe { self_ }
	c.unit = u
	return c
}

pub fn (mut c BarChart) request_layout(mut cx core.LayoutCx) layout.NodeId {
	c.node = cx.tree.node(layout.Style{
		width:  layout.pct(100)
		height: layout.px(f32(c.bars.len) * c.row_h + 4)
	}, [])
	return c.node
}

pub fn (mut c BarChart) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.register_test_id(c.id, bounds)
	for i, b in c.bars {
		cx.register_text(b.label, prim.rect(bounds.x, bounds.y + f32(i) * c.row_h, c.label_w,
			c.row_h))
	}
}

// bar_width is the length of a bar for value v in a track of width w.
pub fn bar_width(v f64, max f64, w f32) f32 {
	return if max > 0 { f32(math.max(v, 0) / max) * w } else { 0 }
}

pub fn (mut c BarChart) paint(bounds prim.Rect, mut cx core.PaintCx) {
	th := kit.theme_of(cx.app)
	style := mono(th, th.text_xs)
	label_style := mono(th, th.text_sm)
	max := nice_max(max_of([c.bars.map(it.value)]))
	track := bounds.width - c.label_w - 64
	for i, b in c.bars {
		y := bounds.y + f32(i) * c.row_h
		ll := cx.text.layout_line(b.label, label_style)
		cx.glyphs(ll, bounds.x, y + c.row_h / 2 + 4, th.p.text_2)
		w := bar_width(b.value, max, track)
		x := bounds.x + c.label_w
		cx.quad(gpu.Quad{
			bounds:     prim.rect(x, y + 5, track, c.row_h - 10)
			background: th.p.line
			radii:      prim.corners(2)
		})
		cx.quad(gpu.Quad{
			bounds:     prim.rect(x, y + 5, prim.max(w, 2), c.row_h - 10)
			background: b.color or { series_color(th, 0) }
			radii:      prim.corners(2)
		})
		vl := cx.text.layout_line('${short(b.value)}${c.unit}', style)
		cx.glyphs(vl, x + track + 8, y + c.row_h / 2 + 4, th.p.text_3)
	}
}

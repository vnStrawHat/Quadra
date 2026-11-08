// Sparkline: an inline trend, a 1 px line over an area at 12 %, sized by its parent.
module viz

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.layout

pub struct Sparkline {
pub mut:
	values []f64
	color  ?prim.Color // default: the theme's first series color
	max    f64         // 0 = the largest value
	width  f32 = 120
	height f32 = 24
mut:
	node layout.NodeId
}

pub fn sparkline(values []f64) &Sparkline {
	return &Sparkline{
		values: values
	}
}

pub fn (self_ &Sparkline) size(w f32, h f32) &Sparkline {
	mut s := unsafe { self_ }
	s.width, s.height = w, h
	return s
}

pub fn (self_ &Sparkline) color(c prim.Color) &Sparkline {
	mut s := unsafe { self_ }
	s.color = c
	return s
}

pub fn (mut s Sparkline) request_layout(mut cx core.LayoutCx) layout.NodeId {
	s.node = cx.tree.node(layout.Style{
		width:  layout.px(s.width)
		height: layout.px(s.height)
	}, [])
	return s.node
}

pub fn (mut s Sparkline) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {}

pub fn (mut s Sparkline) paint(bounds prim.Rect, mut cx core.PaintCx) {
	th := kit.theme_of(cx.app)
	c := s.color or { series_color(th, 0) }
	max := if s.max > 0 { s.max } else { max_of([s.values]) * 1.1 }
	paint_line(s.values, bounds, max, c, 0.12, 1.5, mut cx)
}

// paint_line draws a series one device pixel column at a time, on the pixel grid
// (fractional columns would show seams): the area below as quads at area_alpha (0 = none),
// then the line as column primitives the shader antialiases exactly.
pub fn paint_line(values []f64, plot prim.Rect, max f64, c prim.Color, area_alpha f32, thick f32, mut cx core.PaintCx) {
	s := cx.scale
	x0 := f32(math.round(plot.x * s))
	w := int(math.round(plot.right() * s) - x0)
	spans := line_spans(values, w, plot.height * s, max, thick * s)
	if area_alpha > 0 {
		fill := c.alpha(c.a * area_alpha)
		for x, sp in spans {
			cx.quad(gpu.Quad{
				bounds:     prim.rect((x0 + f32(x)) / s, plot.y + sp.y / s, 1 / s,
					plot.height - sp.y / s)
				background: fill
			})
		}
	}
	for x, sp in spans {
		cx.line(gpu.LinePrim{
			bounds:    prim.rect((x0 + f32(x)) / s, plot.y + (sp.top - 1) / s, 1 / s, (sp.bottom -
				sp.top + 2) / s)
			color:     c
			thickness: sp.thick / s
			rise:      (sp.bottom - sp.top - sp.thick) / s
			column:    true
		})
	}
}

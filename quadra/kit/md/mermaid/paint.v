// Painting a Drawing: boxes as quads, lines and outlines as GPU strokes, filled shapes as
// cached coverage bitmaps (snapped to device pixels, so the cache key does not depend on
// where the diagram is), arrowheads as fans of strokes (no bitmaps for them at all).
module mermaid

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.text
import quadra.vector

fn ink_color(t &kit.Theme, i Ink) prim.Color {
	p := t.p
	return match i {
		.none { prim.transparent }
		.panel { p.panel }
		.canvas { p.canvas }
		.line { p.line_strong }
		.text { p.text }
		.text_2 { p.text_2 }
		.accent { p.accent }
		.frame { p.accent.alpha(0.05) }
		.on_accent { p.text_on_accent }
		.note { p.accent.alpha(0.12) }
	}
}

fn paint_drawing(d &Drawing, o prim.Point, t &kit.Theme, mut cx core.PaintCx) {
	for op in d.ops {
		color := ink_color(t, op.ink)
		match op.kind {
			.box {
				cx.quad(gpu.Quad{
					bounds:        op.rect.translate(o.x, o.y)
					background:    ink_color(t, op.bg)
					border_widths: if op.border == .none { prim.Edges{} } else { prim.edges(1) }
					border_color:  ink_color(t, op.border)
					radii:         prim.corners(op.radius)
				})
			}
			.poly {
				pts := op.pts.map(prim.pt(it.x + o.x, it.y + o.y))
				if op.dash {
					for dash in dashes(pts, 5, 4) {
						cx.stroke(dash, op.width, color, false)
					}
				} else {
					cx.stroke(pts, op.width, color, op.closed)
				}
			}
			.fill {
				fill_poly(op.pts, o, ink_color(t, op.bg), ink_color(t, op.border), mut cx)
			}
			.text {
				l := cx.text.layout_line(op.text, text.TextStyle{
					family: t.font_family
					size:   op.size
					weight: op.weight
				})
				cx.glyphs(l, op.rect.x + o.x, op.rect.y + o.y, color)
			}
			.head {
				paint_head(prim.pt(op.pts[0].x + o.x, op.pts[0].y + o.y), prim.pt(op.pts[1].x + o.x,

					op.pts[1].y + o.y), op.head, color, mut cx)
			}
		}
	}
}

// fill_poly fills a polygon, moved by under half a device pixel so it starts on one; the
// border follows the same move.
fn fill_poly(pts []prim.Point, o prim.Point, bg prim.Color, border prim.Color, mut cx core.PaintCx) {
	s := cx.scale
	mut x0, mut y0 := f32(1e9), f32(1e9)
	mut x1, mut y1 := f32(-1e9), f32(-1e9)
	for q in pts {
		x0, y0 = prim.min(x0, q.x + o.x), prim.min(y0, q.y + o.y)
		x1, y1 = prim.max(x1, q.x + o.x), prim.max(y1, q.y + o.y)
	}
	sx, sy := f32(math.round(x0 * s)) / s, f32(math.round(y0 * s)) / s // snapped origin
	local := pts.map(prim.pt(it.x + o.x - x0, it.y + o.y - y0))
	mut path := vector.new_path()
	for i, q in local {
		if i == 0 {
			path.move_to(q.x, q.y)
		} else {
			path.line_to(q.x, q.y)
		}
	}
	path.close()
	pad := 1
	w, h := int(math.ceil((x1 - x0) * s)) + 2 * pad, int(math.ceil((y1 - y0) * s)) + 2 * pad
	if !bg.is_transparent() && w > 0 && h > 0 && w <= 4096 && h <= 4096 {
		key := 'mermaid:${path.hash()}:${s}:${w}x${h}'
		mut app := cx.app
		entry := app.cached_image(key) or {
			mut cov := vector.new_coverage(w, h)
			cov.fill(path.flatten(s, pad, pad, 0.25))
			app.image_put(key, core.Bitmap{w, h, cov.to_u8(), true})
		}
		cx.sprite(prim.rect(sx - f32(pad) / s, sy - f32(pad) / s, f32(w) / s, f32(h) / s), entry,
			bg)
	}
	if !border.is_transparent() {
		cx.stroke(local.map(prim.pt(it.x + sx, it.y + sy)), 1, border, true)
	}
}

// paint_head draws a head with its tip at tip, the line arriving from `from`.
fn paint_head(tip prim.Point, from prim.Point, h Head, color prim.Color, mut cx core.PaintCx) {
	mut dx, mut dy := tip.x - from.x, tip.y - from.y
	l := f32(math.sqrt(dx * dx + dy * dy))
	if l < 0.01 {
		return
	}
	dx, dy = dx / l, dy / l
	px, py := -dy, dx
	match h {
		.arrow {
			hw := l * 0.45
			n := int(math.ceil(2 * hw / 0.7)) + 1
			for i in 0 .. n {
				k := -hw + 2 * hw * f32(i) / f32(n - 1)
				cx.stroke([tip, prim.pt(from.x + px * k, from.y + py * k)], 1.1, color, false)
			}
		}
		.open {
			for k in [f32(-4.5), 4.5] {
				cx.stroke([prim.pt(tip.x - dx * 9 + px * k, tip.y - dy * 9 + py * k), tip], 1.5,
					color, false)
			}
		}
		.circle {
			r := f32(4)
			c := prim.pt(tip.x - dx * r, tip.y - dy * r)
			cx.quad(gpu.Quad{
				bounds:     prim.rect(c.x - r, c.y - r, 2 * r, 2 * r)
				background: color
				radii:      prim.corners(r)
			})
		}
		.cross {
			c := prim.pt(tip.x - dx * 5, tip.y - dy * 5)
			for k in [f32(1), -1] {
				cx.stroke([prim.pt(c.x + (dx + px * k) * 4, c.y + (dy + py * k) * 4),
					prim.pt(c.x - (dx + px * k) * 4, c.y - (dy + py * k) * 4)], 1.5, color, false)
			}
		}
		.none {}
	}
}

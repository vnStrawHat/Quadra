// Geometry shared by the diagram kinds: smooth curves through points, clipping a line to a
// node's outline, shortening a line for its arrowhead, dashes, and moving a drawing so it
// starts at the margin.
module mermaid

import math
import quadra.prim
import quadra.vector

// basis is the uniform B-spline through the first and last points that the middle points
// pull on (the classic "basis" curve), flattened to a polyline.
fn basis(pts []prim.Point) []prim.Point {
	if pts.len < 3 {
		return pts
	}
	mut p := vector.new_path()
	p.move_to(pts[0].x, pts[0].y)
	mut a, mut b := pts[0], pts[1]
	for i in 2 .. pts.len {
		c := pts[i]
		if i == 2 {
			p.line_to((5 * a.x + b.x) / 6, (5 * a.y + b.y) / 6)
		}
		p.cubic_to((2 * a.x + b.x) / 3, (2 * a.y + b.y) / 3, (a.x + 2 * b.x) / 3,
			(a.y + 2 * b.y) / 3, (a.x + 4 * b.x + c.x) / 6, (a.y + 4 * b.y + c.y) / 6)
		a, b = b, c
	}
	p.cubic_to((2 * a.x + b.x) / 3, (2 * a.y + b.y) / 3, (a.x + 2 * b.x) / 3, (a.y + 2 * b.y) / 3, (
		a.x + 4 * b.x + b.x) / 6, (a.y + 4 * b.y + b.y) / 6)
	p.line_to(b.x, b.y)
	lines := p.flatten(1, 0, 0, 0.2)
	return if lines.len > 0 { lines[0].points } else { pts }
}

// clip moves from the center of box r toward `to` and stops on the shape's outline.
fn clip(r prim.Rect, shape Shape, to prim.Point) prim.Point {
	c := prim.pt(r.x + r.width / 2, r.y + r.height / 2)
	dx, dy := to.x - c.x, to.y - c.y
	if dx == 0 && dy == 0 {
		return c
	}
	hw, hh := r.width / 2, r.height / 2
	t := match shape {
		.diamond {
			1 / (math.abs(dx) / hw + math.abs(dy) / hh)
		}
		.circle {
			hw / f32(math.sqrt(dx * dx + dy * dy))
		}
		else {
			prim.min(if dx == 0 { f32(1e9) } else { hw / math.abs(dx) }, if dy == 0 {
				f32(1e9)
			} else {
				hh / math.abs(dy)
			})
		}
	}

	return prim.pt(c.x + dx * t, c.y + dy * t)
}

fn dist(a prim.Point, b prim.Point) f32 {
	return f32(math.sqrt((a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)))
}

// cut_end drops the last `len` px of a polyline; it returns the rest and the new end.
fn cut_end(pts []prim.Point, len f32) []prim.Point {
	if len <= 0 || pts.len < 2 {
		return pts
	}
	mut left := len
	for i := pts.len - 1; i > 0; i-- {
		s := dist(pts[i - 1], pts[i])
		if s >= left {
			k := (s - left) / s
			mut out := pts[..i].clone()
			out << prim.pt(pts[i - 1].x + (pts[i].x - pts[i - 1].x) * k, pts[i - 1].y +
				(pts[i].y - pts[i - 1].y) * k)
			return out
		}
		left -= s
	}
	return [pts[0], pts[0]]
}

fn reversed(pts []prim.Point) []prim.Point {
	mut out := []prim.Point{cap: pts.len}
	for i := pts.len - 1; i >= 0; i-- {
		out << pts[i]
	}
	return out
}

// head_len is how far before the tip the line stops for a head.
fn head_len(h Head, width f32) f32 {
	return match h {
		.arrow { 7 + 2 * width }
		.circle { 8 }
		.cross { 5 }
		else { 0 }
	}
}

// dashes splits a polyline into dashes of `on` px separated by `off` px.
fn dashes(pts []prim.Point, on f32, off f32) [][]prim.Point {
	mut out := [][]prim.Point{}
	mut cur := []prim.Point{}
	mut drawing, mut left := true, on
	for i in 0 .. pts.len - 1 {
		mut a := pts[i]
		b := pts[i + 1]
		mut s := dist(a, b)
		if drawing && cur.len == 0 {
			cur << a
		}
		for s > left {
			k := left / s
			p := prim.pt(a.x + (b.x - a.x) * k, a.y + (b.y - a.y) * k)
			if drawing {
				cur << p
				out << cur
				cur = []prim.Point{}
			} else {
				cur = [p]
			}
			s -= left
			a = p
			drawing = !drawing
			left = if drawing { on } else { off }
		}
		left -= s
		if drawing {
			cur << b
		}
	}
	if cur.len > 1 {
		out << cur
	}
	return out
}

// normalize moves the drawing so its content starts at (margin, margin) and sets its size.
fn (mut d Drawing) normalize(margin f32) {
	mut x0, mut y0 := f32(1e9), f32(1e9)
	mut x1, mut y1 := f32(-1e9), f32(-1e9)
	for op in d.ops {
		mut r := op.rect
		if op.kind == .text {
			r = prim.rect(op.rect.x, op.rect.y - op.rect.height * 0.72, op.rect.width,
				op.rect.height)
		}
		if op.kind in [.text, .box] {
			x0, y0 = prim.min(x0, r.x), prim.min(y0, r.y)
			x1, y1 = prim.max(x1, r.x + r.width), prim.max(y1, r.y + r.height)
		}
		for q in op.pts {
			x0, y0 = prim.min(x0, q.x - 6), prim.min(y0, q.y - 6)
			x1, y1 = prim.max(x1, q.x + 6), prim.max(y1, q.y + 6)
		}
	}
	if x1 < x0 {
		return
	}
	dx, dy := margin - x0, margin - y0
	for mut op in d.ops {
		op = Op{
			...op
			rect: op.rect.translate(dx, dy)
			pts:  op.pts.map(prim.pt(it.x + dx, it.y + dy))
		}
	}
	d.width, d.height = x1 - x0 + 2 * margin, y1 - y0 + 2 * margin
}

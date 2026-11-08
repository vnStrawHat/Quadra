// Vector paths and flattening into polylines (pure V).
module vector

import math
import quadra.prim

enum Verb {
	move
	line
	quad
	cubic
	close
}

struct Cmd {
	verb Verb
	pts  []prim.Point
}

pub struct Path {
mut:
	cmds  []Cmd
	start prim.Point
	cur   prim.Point
}

pub fn new_path() Path {
	return Path{}
}

pub fn (mut p Path) move_to(x f32, y f32) {
	p.cmds << Cmd{.move, [prim.pt(x, y)]}
	p.start, p.cur = prim.pt(x, y), prim.pt(x, y)
}

pub fn (mut p Path) line_to(x f32, y f32) {
	p.cmds << Cmd{.line, [prim.pt(x, y)]}
	p.cur = prim.pt(x, y)
}

pub fn (mut p Path) quad_to(cx f32, cy f32, x f32, y f32) {
	p.cmds << Cmd{.quad, [prim.pt(cx, cy), prim.pt(x, y)]}
	p.cur = prim.pt(x, y)
}

pub fn (mut p Path) cubic_to(c1x f32, c1y f32, c2x f32, c2y f32, x f32, y f32) {
	p.cmds << Cmd{.cubic, [prim.pt(c1x, c1y), prim.pt(c2x, c2y),
		prim.pt(x, y)]}
	p.cur = prim.pt(x, y)
}

pub fn (mut p Path) close() {
	p.cmds << Cmd{.close, []}
	p.cur = p.start
}

pub fn (p &Path) current() prim.Point {
	return p.cur
}

// arc_to appends an SVG elliptical arc (endpoint parameterization) as cubic curves.
pub fn (mut p Path) arc_to(rx_ f32, ry_ f32, rotation f32, large bool, sweep bool, x f32, y f32) {
	x1, y1 := p.cur.x, p.cur.y
	if rx_ == 0 || ry_ == 0 || (x1 == x && y1 == y) {
		p.line_to(x, y)
		return
	}
	phi := f64(rotation) * math.pi / 180
	cphi, sphi := math.cos(phi), math.sin(phi)
	dx, dy := f64(x1 - x) / 2, f64(y1 - y) / 2
	x1p := cphi * dx + sphi * dy
	y1p := -sphi * dx + cphi * dy
	mut rx, mut ry := math.abs(f64(rx_)), math.abs(f64(ry_))
	lambda := (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry)
	if lambda > 1 {
		rx *= math.sqrt(lambda)
		ry *= math.sqrt(lambda)
	}
	num := rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p
	den := rx * rx * y1p * y1p + ry * ry * x1p * x1p
	mut coef := if den == 0 { 0.0 } else { math.sqrt(math.max(0.0, num / den)) }
	if large == sweep {
		coef = -coef
	}
	cxp := coef * rx * y1p / ry
	cyp := -coef * ry * x1p / rx
	cx := cphi * cxp - sphi * cyp + f64(x1 + x) / 2
	cy := sphi * cxp + cphi * cyp + f64(y1 + y) / 2
	th1 := angle(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry)
	mut dth := angle((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry)
	if !sweep && dth > 0 {
		dth -= 2 * math.pi
	} else if sweep && dth < 0 {
		dth += 2 * math.pi
	}
	segs := int(math.ceil(math.abs(dth) / (math.pi / 2)))
	step := dth / f64(segs)
	k := 4.0 / 3.0 * math.tan(step / 4)
	f := ArcFrame{rx, ry, cphi, sphi, cx, cy}
	mut t := th1
	for _ in 0 .. segs {
		c1, s1 := math.cos(t), math.sin(t)
		c2, s2 := math.cos(t + step), math.sin(t + step)
		a := f.at(c1 - k * s1, s1 + k * c1)
		b := f.at(c2 + k * s2, s2 - k * c2)
		e := f.at(c2, s2)
		p.cubic_to(a.x, a.y, b.x, b.y, e.x, e.y)
		t += step
	}
}

// ArcFrame maps a point of the unit circle onto the arc's ellipse.
struct ArcFrame {
	rx   f64
	ry   f64
	cphi f64
	sphi f64
	cx   f64
	cy   f64
}

fn (f ArcFrame) at(ex f64, ey f64) prim.Point {
	return prim.pt(f32(f.cphi * ex * f.rx - f.sphi * ey * f.ry + f.cx), f32(f.sphi * ex * f.rx +
		f.cphi * ey * f.ry + f.cy))
}

fn angle(ux f64, uy f64, vx f64, vy f64) f64 {
	return math.atan2(ux * vy - uy * vx, ux * vx + uy * vy)
}

pub fn (mut p Path) rect(x f32, y f32, w f32, h f32) {
	p.move_to(x, y)
	p.line_to(x + w, y)
	p.line_to(x + w, y + h)
	p.line_to(x, y + h)
	p.close()
}

pub fn (mut p Path) rounded_rect(x f32, y f32, w f32, h f32, r_ f32) {
	r := prim.min(r_, prim.min(w, h) / 2)
	if r <= 0 {
		p.rect(x, y, w, h)
		return
	}
	p.move_to(x + r, y)
	p.line_to(x + w - r, y)
	p.arc_to(r, r, 0, false, true, x + w, y + r)
	p.line_to(x + w, y + h - r)
	p.arc_to(r, r, 0, false, true, x + w - r, y + h)
	p.line_to(x + r, y + h)
	p.arc_to(r, r, 0, false, true, x, y + h - r)
	p.line_to(x, y + r)
	p.arc_to(r, r, 0, false, true, x + r, y)
	p.close()
}

pub fn (mut p Path) ellipse(cx f32, cy f32, rx f32, ry f32) {
	p.move_to(cx + rx, cy)
	p.arc_to(rx, ry, 0, false, true, cx - rx, cy)
	p.arc_to(rx, ry, 0, false, true, cx + rx, cy)
	p.close()
}

pub struct Polyline {
pub mut:
	points []prim.Point
	closed bool
}

// flatten converts curves to line segments with the given tolerance, after mapping
// every point through scale and offset (user units -> pixels).
pub fn (p &Path) flatten(scale f32, ox f32, oy f32, tolerance f32) []Polyline {
	mut out := []Polyline{}
	mut cur := Polyline{}
	mut last := prim.Point{}
	// No closure for the mapping: flatten runs in paint (canvas, GPU strokes), where a
	// closure has no owner and would never be freed.
	for c in p.cmds {
		match c.verb {
			.move {
				if cur.points.len > 1 {
					out << cur
				}
				last = prim.pt(c.pts[0].x * scale + ox, c.pts[0].y * scale + oy)
				cur = Polyline{
					points: [last]
				}
			}
			.line {
				last = prim.pt(c.pts[0].x * scale + ox, c.pts[0].y * scale + oy)
				cur.points << last
			}
			.quad, .cubic {
				ctrl := c.pts.map(prim.pt(it.x * scale + ox, it.y * scale + oy))
				p0 := last
				mut span := f32(0)
				for q in ctrl {
					span = prim.max(span, prim.max(f32_abs(q.x - p0.x), f32_abs(q.y - p0.y)))
				}
				n := int(prim.clamp(f32(math.sqrt(span / tolerance)) * 2, 2, 64))
				for i in 1 .. n + 1 {
					t := f32(i) / f32(n)
					cur.points << if c.verb == .quad {
						quad_at(p0, ctrl[0], ctrl[1], t)
					} else {
						cubic_at(p0, ctrl[0], ctrl[1], ctrl[2], t)
					}
				}
				last = ctrl.last()
			}
			.close {
				cur.closed = true
				if cur.points.len > 1 {
					out << cur
				}
				last = if cur.points.len > 0 { cur.points[0] } else { last }
				cur = Polyline{
					points: [last]
				}
			}
		}
	}
	if cur.points.len > 1 {
		out << cur
	}
	return out
}

fn quad_at(a prim.Point, b prim.Point, c prim.Point, t f32) prim.Point {
	u := 1 - t
	return prim.pt(u * u * a.x + 2 * u * t * b.x + t * t * c.x, u * u * a.y + 2 * u * t * b.y +
		t * t * c.y)
}

fn cubic_at(a prim.Point, b prim.Point, c prim.Point, d prim.Point, t f32) prim.Point {
	u := 1 - t
	return prim.pt(u * u * u * a.x + 3 * u * u * t * b.x + 3 * u * t * t * c.x + t * t * t * d.x,

		u * u * u * a.y + 3 * u * u * t * b.y + 3 * u * t * t * c.y + t * t * t * d.y)
}

@[inline]
fn f32_abs(v f32) f32 {
	return if v < 0 { -v } else { v }
}

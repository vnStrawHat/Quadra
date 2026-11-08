// CPU rasterization to 8-bit coverage: fills by signed-area accumulation (exact
// antialiasing), strokes by distance to segments (round caps and joins for free).
module vector

import math
import quadra.prim

pub struct Coverage {
pub:
	width  int
	height int
pub mut:
	data []f32
}

pub fn new_coverage(w int, h int) Coverage {
	return Coverage{
		width:  w
		height: h
		data:   []f32{len: (w + 1) * h}
	}
}

// fill accumulates the area of closed polylines (non-zero for non-overlapping shapes).
pub fn (mut c Coverage) fill(lines []Polyline) {
	mut acc := []f32{len: (c.width + 2) * (c.height + 1)}
	for pl in lines {
		n := pl.points.len
		for i in 0 .. n {
			a := pl.points[i]
			b := pl.points[(i + 1) % n]
			// Clamping x projects off-bitmap parts onto the edges, which keeps the area
			// inside exact: coverage left of the bitmap lands in column 0.
			w := f32(c.width)
			accumulate_line(mut acc, c.width + 2, c.height, prim.pt(prim.clamp(a.x, 0, w), a.y), prim.pt(prim.clamp(b.x,
				0, w), b.y))
		}
	}
	for y in 0 .. c.height {
		mut sum := f32(0)
		for x in 0 .. c.width {
			sum += acc[y * (c.width + 2) + x]
			v := if sum < 0 { -sum } else { sum }
			idx := y * (c.width + 1) + x
			c.data[idx] = prim.min(1, prim.max(c.data[idx], prim.min(v, 1)))
		}
	}
}

// accumulate_line adds the signed area contribution of one edge (font-rs style).
fn accumulate_line(mut acc []f32, stride int, h int, p0 prim.Point, p1 prim.Point) {
	if p0.y == p1.y {
		return
	}
	dir := f32(if p0.y < p1.y { 1 } else { -1 })
	a, b := if p0.y < p1.y { p0, p1 } else { p1, p0 }
	dxdy := (b.x - a.x) / (b.y - a.y)
	mut x := a.x
	y0 := int(prim.max(0, f32(math.floor(a.y))))
	if a.y < 0 {
		x -= a.y * dxdy
	}
	for y in y0 .. int(prim.min(f32(h), f32(math.ceil(b.y)))) {
		linestart := y * stride
		dy := prim.min(f32(y + 1), b.y) - prim.max(f32(y), a.y)
		xnext := x + dxdy * dy
		d := dy * dir
		x0, x1 := if x < xnext { x, xnext } else { xnext, x }
		x0floor := f32(math.floor(x0))
		x0i := int(x0floor)
		x1ceil := f32(math.ceil(x1))
		x1i := int(x1ceil)
		if x1i <= x0i + 1 {
			xmf := 0.5 * (x + xnext) - x0floor
			add(mut acc, linestart + x0i, d - d * xmf, stride)
			add(mut acc, linestart + x0i + 1, d * xmf, stride)
		} else {
			s := 1 / (x1 - x0)
			x0f := x0 - x0floor
			a0 := 0.5 * s * (1 - x0f) * (1 - x0f)
			x1f := x1 - x1ceil + 1
			am := 0.5 * s * x1f * x1f
			add(mut acc, linestart + x0i, d * a0, stride)
			if x1i == x0i + 2 {
				add(mut acc, linestart + x0i + 1, d * (1 - a0 - am), stride)
			} else {
				a1 := s * (1.5 - x0f)
				add(mut acc, linestart + x0i + 1, d * (a1 - a0), stride)
				for xi in x0i + 2 .. x1i - 1 {
					add(mut acc, linestart + xi, d * s, stride)
				}
				a2 := a1 + f32(x1i - x0i - 3) * s
				add(mut acc, linestart + x1i - 1, d * (1 - a2 - am), stride)
			}
			add(mut acc, linestart + x1i, d * am, stride)
		}
		x = xnext
	}
}

@[inline]
fn add(mut acc []f32, i int, v f32, _ int) {
	if i >= 0 && i < acc.len {
		acc[i] += v
	}
}

// stroke draws polylines with round caps and joins, width in pixels.
pub fn (mut c Coverage) stroke(lines []Polyline, width f32) {
	hw := width / 2
	for pl in lines {
		n := pl.points.len
		segs := if pl.closed { n } else { n - 1 }
		for i in 0 .. segs {
			c.capsule(pl.points[i], pl.points[(i + 1) % n], hw)
		}
		if n == 1 {
			c.capsule(pl.points[0], pl.points[0], hw)
		}
	}
}

fn (mut c Coverage) capsule(a prim.Point, b prim.Point, hw f32) {
	x0 := int(prim.max(0, prim.min(a.x, b.x) - hw - 1))
	y0 := int(prim.max(0, prim.min(a.y, b.y) - hw - 1))
	x1 := int(prim.min(f32(c.width - 1), prim.max(a.x, b.x) + hw + 1))
	y1 := int(prim.min(f32(c.height - 1), prim.max(a.y, b.y) + hw + 1))
	abx, aby := b.x - a.x, b.y - a.y
	len2 := abx * abx + aby * aby
	for y in y0 .. y1 + 1 {
		for x in x0 .. x1 + 1 {
			px, py := f32(x) + 0.5 - a.x, f32(y) + 0.5 - a.y
			t := if len2 > 0 { prim.clamp((px * abx + py * aby) / len2, 0, 1) } else { 0 }
			dx, dy := px - abx * t, py - aby * t
			d := f32(math.sqrt(dx * dx + dy * dy))
			v := prim.clamp(hw + 0.5 - d, 0, 1)
			idx := y * (c.width + 1) + x
			if v > c.data[idx] {
				c.data[idx] = v
			}
		}
	}
}

// to_u8 converts coverage to an 8-bit mask.
pub fn (c &Coverage) to_u8() []u8 {
	mut out := []u8{len: c.width * c.height}
	for y in 0 .. c.height {
		for x in 0 .. c.width {
			out[y * c.width + x] = u8(prim.clamp(c.data[y * (c.width + 1) + x], 0, 1) * 255 + 0.5)
		}
	}
	return out
}

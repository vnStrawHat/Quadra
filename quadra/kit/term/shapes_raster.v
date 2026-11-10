// Curves and diagonals (arcs, ╱╲╳, powerline): rasterized into coverage rects with a
// fractional alpha, sampled on a grid that is exactly mirror symmetric (specs/17).
module term

import math

enum RasterKind {
	rounded
	diagonal
	cross
	triangle
	chevron
	half_disc
	half_disc_outline
	corner
	corner_outline
}

struct RasterSpec {
	kind RasterKind
	half f32
	// rounded corner
	r  f32
	hx f32
	hy f32
}

const raster_samples = 4
const aa_pixel_cap = 64 * 128 // above this a cell takes one sample per pixel

// rasterize covers `spec` over the cell; the sample at offset (u, v) from the cell center,
// moved by (sx, sy) and then mirrored, decides each sub-pixel.
fn rasterize(g &Geometry, m Mirror, sx f32, sy f32, spec RasterSpec, mut out []ShapeRect) {
	n := if g.w * g.h <= aa_pixel_cap { raster_samples } else { 1 }
	total := n * n
	den := f32(2 * n)
	base := out.len
	for y in 0 .. g.h {
		mut run_start, mut run_count := 0, 0
		for x in 0 .. g.w {
			mut count := 0
			for j in 0 .. n {
				mut v := f32(2 * n * y + 2 * j + 1 - n * g.h) / den - sy
				if m.y {
					v = -v
				}
				for i in 0 .. n {
					mut u := f32(2 * n * x + 2 * i + 1 - n * g.w) / den - sx
					if m.x {
						u = -u
					}
					if raster_inside(g, spec, u, v) {
						count++
					}
				}
			}
			if count != run_count {
				emit_run(mut out, base, run_start, y, x - run_start, run_count, total)
				run_start, run_count = x, count
			}
		}
		emit_run(mut out, base, run_start, y, g.w - run_start, run_count, total)
	}
}

fn emit_run(mut out []ShapeRect, base int, x int, y int, w int, count int, total int) {
	if count == 0 || w <= 0 {
		return
	}
	alpha := f32(count) / f32(total)
	for i := out.len - 1; i >= base; i-- {
		r := out[i]
		if r.x == x && r.w == w && r.y + r.h == y && r.alpha == alpha {
			out[i] = ShapeRect{r.x, r.y, r.w, r.h + 1, r.alpha}
			return
		}
	}
	out << ShapeRect{x, y, w, 1, alpha}
}

@[inline]
fn fsq(v f32) f32 {
	return v * v
}

fn segment_distance_sq(u f32, v f32, ax f32, ay f32, bx f32, by f32) f32 {
	abx, aby := bx - ax, by - ay
	apx, apy := u - ax, v - ay
	len_sq := abx * abx + aby * aby
	mut t := f32(0)
	if len_sq > 0 {
		t = fclamp((apx * abx + apy * aby) / len_sq, 0, 1)
	}
	dx, dy := apx - t * abx, apy - t * aby
	return dx * dx + dy * dy
}

fn in_elliptic_band(du f32, dv f32, a0 f32, b0 f32, a1 f32, b1 f32) bool {
	outside_hole := a0 <= 0 || b0 <= 0 || fsq(du / a0) + fsq(dv / b0) >= 1
	return outside_hole && fsq(du / a1) + fsq(dv / b1) <= 1
}

fn on_diagonal(g &Geometry, u f32, v f32, half f32) bool {
	w, h := f32(g.w), f32(g.h)
	return f32(math.abs(u * h - v * w)) <= half * f32(math.sqrt(w * w + h * h))
}

fn raster_inside(g &Geometry, s RasterSpec, u f32, v f32) bool {
	w, h, cx, cy := f32(g.w), f32(g.h), g.cx, g.cy
	match s.kind {
		.rounded {
			if (v >= s.r && f32(math.abs(u)) <= s.hx) || (u >= s.r && f32(math.abs(v)) <= s.hy) {
				return true
			}
			du, dv := u - s.r, v - s.r
			return du <= 0 && dv <= 0
				&& in_elliptic_band(du, dv, s.r - s.hx, s.r - s.hy, s.r + s.hx, s.r + s.hy)
		}
		.diagonal {
			return on_diagonal(g, u, v, s.half)
		}
		.cross {
			return on_diagonal(g, u, v, s.half) || on_diagonal(g, -u, v, s.half)
		}
		.triangle {
			return f32(math.abs(v)) * w <= cy * (cx - u)
		}
		.chevron {
			return segment_distance_sq(u, f32(math.abs(v)), -cx, cy, cx - s.half, 0) <= s.half * s.half
		}
		.half_disc {
			return fsq((u + cx) / w) + fsq(v / cy) <= 1
		}
		.half_disc_outline {
			t := s.half * 2
			return in_elliptic_band(u + cx, v, w - t, cy - t, w, cy)
		}
		.corner {
			return v * w >= u * h
		}
		.corner_outline {
			return on_diagonal(g, u, v, s.half)
		}
	}
}

fn rounded_mirror(code u32) ?Mirror {
	return match code {
		0x256d {
			Mirror{}
		}
		0x256e {
			Mirror{
				x: true
			}
		}
		0x256f {
			Mirror{
				x: true
				y: true
			}
		}
		0x2570 {
			Mirror{
				y: true
			}
		}
		else {
			none
		}
	}
}

fn rounded_corner(m Mirror, g &Geometry, mut out []ShapeRect) {
	hx := f32(g.joint(.x, g.t.light).len()) / 2
	hy := f32(g.joint(.y, g.t.light).len()) / 2
	sx := g.axis_center(.x) - g.cx
	sy := g.axis_center(.y) - g.cy
	a, b := g.cx - hx - sx, g.cy - hy - sy
	mut r := if a < b { a } else { b }
	if r < 0 {
		r = 0
	}
	rasterize(g, m, sx, sy, RasterSpec{
		kind: .rounded
		r:    r
		hx:   hx
		hy:   hy
	}, mut out)
}

// powerline_kinds: (shape, x mirror, y mirror) for U+E0B0..E0BF in order.
const powerline_kinds = [
	RasterKind.triangle,
	.chevron,
	.triangle,
	.chevron,
	.half_disc,
	.half_disc_outline,
	.half_disc,
	.half_disc_outline,
	.corner,
	.corner_outline,
	.corner,
	.corner_outline,
	.corner,
	.corner_outline,
	.corner,
	.corner_outline,
]

fn powerline(code u32, g &Geometry, mut out []ShapeRect) {
	i := int(code - 0xe0b0)
	if i < 0 || i >= powerline_kinds.len {
		return
	}
	// E0B0..E0B7: pairs mirror x (B2, B3, B6, B7). E0B8..E0BF: ID, MX, MY, MXY per group of two.
	mirror := if i < 8 {
		Mirror{
			x: i & 2 != 0
		}
	} else {
		Mirror{i & 2 != 0, i & 4 != 0}
	}
	rasterize(g, mirror, 0, 0, RasterSpec{
		kind: powerline_kinds[i]
		half: f32(g.t.light) / 2
	}, mut out)
}

// Geometry for the code points drawn as shapes instead of font glyphs (specs/17, View): box
// drawing, block elements, U+25AC, braille and powerline. Fonts disagree about these glyphs and
// their strokes rarely meet across cell boundaries. Everything is computed once per
// (character, device cell size, weight) in cell-relative device pixels; see ShapeCache.
module term

import math

// ShapeRect is a rectangle in cell-relative device pixels with coverage `alpha`.
pub struct ShapeRect {
pub:
	x     int
	y     int
	w     int
	h     int
	alpha f32 = 1
}

struct StrokeThickness {
	light    int
	heavy    int
	rail     int
	gap      int
	dash_gap int
}

fn stroke_thickness(w_px int, font_weight f32) StrokeThickness {
	w := f32(int_max(w_px, 1))
	k := fclamp(font_weight / 400, 1, 2.25)
	base_light := int_max(rnd(w / 8), 1)
	light := int_max(rnd(w / 8 * k), 1)
	heavy := rnd(w / 3 * k)
	return StrokeThickness{
		light:    light
		heavy:    int_max(heavy, light + 2)
		rail:     light
		gap:      light
		dash_gap: base_light
	}
}

@[inline]
fn rnd(v f32) int {
	return int(math.round(v))
}

@[inline]
fn fclamp(v f32, lo f32, hi f32) f32 {
	return if v < lo {
		lo
	} else if v > hi {
		hi
	} else {
		v
	}
}

@[inline]
fn int_max(a int, b int) int {
	return if a > b { a } else { b }
}

@[inline]
fn int_min(a int, b int) int {
	return if a < b { a } else { b }
}

// round_ties_toward rounds, breaking ties toward `center` so reflection commutes with rounding.
fn round_ties_toward(v f32, center f32) int {
	lower := f32(math.floor(v))
	frac := v - lower
	diff := if frac >= 0.5 { frac - 0.5 } else { 0.5 - frac }
	up := if diff <= 1e-4 { v < center } else { frac > 0.5 }
	return int(if up {
		lower + 1
	} else {
		lower
	})
}

enum SnapAnchor {
	fixed
	nearest
}

struct Interval {
mut:
	lo int
	hi int
}

fn (i Interval) empty() bool {
	return i.hi <= i.lo
}

fn (i Interval) len() int {
	return if i.hi > i.lo { i.hi - i.lo } else { 0 }
}

fn (a Interval) touches(b Interval) bool {
	return a.lo <= b.hi && b.lo <= a.hi
}

fn (a Interval) join(b Interval) Interval {
	return Interval{int_min(a.lo, b.lo), int_max(a.hi, b.hi)}
}

// snap_interval snaps a centered extent to whole device pixels, keeping the thickness.
fn snap_interval(center f32, half f32, size int, anchor SnapAnchor) Interval {
	mut thickness := int_max(rnd(2 * half), 1)
	mut center_halves := round_ties_toward(2 * center, f32(size))
	if (center_halves - thickness) % 2 != 0 {
		if anchor == .fixed {
			center_halves-- // toward the top/left, for every stroke alike
		} else if center_halves < size {
			center_halves--
		} else if center_halves > size {
			center_halves++
		} else {
			thickness++ // exactly on the cell center: widen, to stay symmetric
		}
	}
	mut lo := (center_halves - thickness) / 2
	lo = int_max(int_min(lo, int_max(size - thickness, 0)), int_min(size - thickness, 0))
	return Interval{int_max(lo, 0), int_min(lo + thickness, size)}
}

enum Axis {
	x
	y
}

fn (a Axis) other() Axis {
	return if a == .x { Axis.y } else { Axis.x }
}

// Geometry is the cell a shape is built in: its size, center and thicknesses.
struct Geometry {
	w           int
	h           int
	cx          f32
	cy          f32
	t           StrokeThickness
	rail_offset f32
}

fn new_geometry(w_ int, h_ int, font_weight f32) Geometry {
	w, h := int_max(w_, 1), int_max(h_, 1)
	t := stroke_thickness(w, font_weight)
	return Geometry{
		w:           w
		h:           h
		cx:          f32(w) / 2
		cy:          f32(h) / 2
		t:           t
		rail_offset: f32(t.rail + t.gap) / 2
	}
}

fn (g &Geometry) center(axis Axis) f32 {
	return if axis == .x { g.cx } else { g.cy }
}

fn (g &Geometry) size(axis Axis) int {
	return if axis == .x { g.w } else { g.h }
}

fn (g &Geometry) joint(axis Axis, thickness int) Interval {
	return snap_interval(g.center(axis), f32(thickness) / 2, g.size(axis), .fixed)
}

fn (g &Geometry) axis_center(axis Axis) f32 {
	j := g.joint(axis, g.t.light)
	return f32(j.lo + j.hi) / 2
}

// rail is one rail of a double line; side is -1 or +1.
fn (g &Geometry) rail(axis Axis, side int) Interval {
	return snap_interval(g.axis_center(axis) + f32(side) * g.rail_offset, f32(g.t.rail) / 2,
		g.size(axis), .nearest)
}

fn (g &Geometry) snap_edge(axis Axis, value f32) int {
	return clamp(round_ties_toward(value, g.center(axis)), 0, g.size(axis))
}

fn push_rect(mut out []ShapeRect, x Interval, y Interval) {
	if !x.empty() && !y.empty() {
		out << ShapeRect{x.lo, y.lo, x.len(), y.len(), 1}
	}
}

// shape_index is a dense index of the drawn code points, -1 for any other.
@[inline]
pub fn shape_index(c u32) int {
	if c >= 0x2500 && c <= 0x259f {
		return int(c - 0x2500)
	}
	if c == 0x25ac {
		return 160
	}
	if c >= 0x2800 && c <= 0x28ff {
		return 161 + int(c - 0x2800)
	}
	if c >= 0xe0b0 && c <= 0xe0bf {
		return 417 + int(c - 0xe0b0)
	}
	return -1
}

pub const shape_count = 433

// shape_quads appends the rects of a code point (font_weight on the CSS scale).
pub fn shape_quads(code u32, w int, h int, font_weight f32, mut out []ShapeRect) {
	if shape_index(code) < 0 {
		return
	}
	g := new_geometry(w, h, font_weight)
	half := f32(g.t.light) / 2
	if code == 0x2571 {
		rasterize(g, Mirror{ x: true }, 0, 0, RasterSpec{
			kind: .diagonal
			half: half
		}, mut out)
	} else if code == 0x2572 {
		rasterize(g, Mirror{}, 0, 0, RasterSpec{ kind: .diagonal, half: half }, mut out)
	} else if code == 0x2573 {
		rasterize(g, Mirror{}, 0, 0, RasterSpec{ kind: .cross, half: half }, mut out)
	} else if code >= 0x2500 && code <= 0x257f {
		arms := box_arms[code - 0x2500]
		if arms != 0 {
			build_arms(arms_of(arms), mirror_of(arms), g, mut out)
		} else if dash := dash_def(code) {
			build_dashes(dash, g, mut out)
		} else if m := rounded_mirror(code) {
			rounded_corner(m, g, mut out)
		}
	} else if code == 0x2591 {
		build_shade(.light, g, mut out)
	} else if code == 0x2592 {
		build_shade(.medium, g, mut out)
	} else if code == 0x2593 {
		build_shade(.dark, g, mut out)
	} else if code <= 0x259f || code == 0x25ac {
		build_block(code, g, mut out)
	} else if code <= 0x28ff {
		build_braille(code, g, mut out)
	} else {
		powerline(code, g, mut out)
	}
}

// ShapeCache holds the rects of each code point for the current cell size and weight.
pub struct ShapeCache {
mut:
	w      int
	h      int
	weight f32
	rects  [][]ShapeRect
	done   []bool
}

// rects_of returns the cached rects; a size or weight change rebuilds lazily.
pub fn (mut c ShapeCache) rects_of(idx int, code u32, w int, h int, weight f32) []ShapeRect {
	if c.w != w || c.h != h || c.weight != weight || c.rects.len == 0 {
		c.w, c.h, c.weight = w, h, weight
		c.rects = [][]ShapeRect{len: shape_count}
		c.done = []bool{len: shape_count}
	}
	if !c.done[idx] {
		mut out := []ShapeRect{}
		shape_quads(code, w, h, weight, mut out)
		c.rects[idx] = out
		c.done[idx] = true
	}
	return c.rects[idx]
}

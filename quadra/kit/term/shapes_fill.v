// Block elements, shades and braille (specs/17).
module term

fn eighth(k int, size int) int {
	v := rnd(f32(k * size) / 8)
	return int_min(int_max(v, 1), size)
}

const q_ul = 1
const q_ur = 2
const q_ll = 4
const q_lr = 8

fn quadrant_mask(code u32) int {
	return match code {
		0x2596 { q_ll }
		0x2597 { q_lr }
		0x2598 { q_ul }
		0x2599 { q_ul | q_ll | q_lr }
		0x259a { q_ul | q_lr }
		0x259b { q_ul | q_ur | q_ll }
		0x259c { q_ul | q_ur | q_lr }
		0x259d { q_ur }
		0x259e { q_ur | q_ll }
		0x259f { q_ur | q_ll | q_lr }
		else { 0 }
	}
}

fn build_block(code u32, g &Geometry, mut out []ShapeRect) {
	full_x, full_y := Interval{0, g.w}, Interval{0, g.h}
	if code == 0x2580 {
		push_rect(mut out, full_x, Interval{0, eighth(4, g.h)})
	} else if code == 0x2594 {
		push_rect(mut out, full_x, Interval{0, eighth(1, g.h)})
	} else if code >= 0x2581 && code <= 0x2588 {
		push_rect(mut out, full_x, Interval{g.h - eighth(int(code - 0x2580), g.h), g.h})
	} else if code >= 0x2589 && code <= 0x258f {
		push_rect(mut out, Interval{0, eighth(8 - int(code - 0x2588), g.w)}, full_y)
	} else if code == 0x2590 {
		push_rect(mut out, Interval{g.w - eighth(4, g.w), g.w}, full_y)
	} else if code == 0x2595 {
		push_rect(mut out, Interval{g.w - eighth(1, g.w), g.w}, full_y)
	} else if code >= 0x2596 && code <= 0x259f {
		// Half extents round up so the four quadrants always cover the whole cell.
		half_w, half_h := (g.w + 1) / 2, (g.h + 1) / 2
		mask := quadrant_mask(code)
		left, right := Interval{0, half_w}, Interval{g.w - half_w, g.w}
		top, bottom := Interval{0, half_h}, Interval{g.h - half_h, g.h}
		if mask & q_ul != 0 {
			push_rect(mut out, left, top)
		}
		if mask & q_ur != 0 {
			push_rect(mut out, right, top)
		}
		if mask & q_ll != 0 {
			push_rect(mut out, left, bottom)
		}
		if mask & q_lr != 0 {
			push_rect(mut out, right, bottom)
		}
	} else if code == 0x25ac {
		push_rect(mut out, full_x, snap_interval(g.cy, f32(g.h) / 4, g.h, .nearest))
	}
}

enum Shade {
	light
	medium
	dark
}

fn build_shade(shade Shade, g &Geometry, mut out []ShapeRect) {
	pitch := int_max(rnd(f32(g.w) / 3), 2)
	tiles_x := ((g.w + pitch - 1) / pitch) | 1 // odd tile counts stay symmetric
	tiles_y := ((g.h + pitch - 1) / pitch) | 1
	for j in 0 .. tiles_y {
		y := Interval{tile_edge(g, .y, j, tiles_y, pitch), tile_edge(g, .y, j + 1, tiles_y, pitch)}
		if y.empty() {
			continue
		}
		mut run := Interval{0, -1}
		for i in 0 .. tiles_x {
			on := match shade {
				.light { (i + j) % 2 == 0 && j % 2 == 0 }
				.medium { (i + j) % 2 == 0 }
				.dark { !((i + j) % 2 == 0 && j % 2 == 1) }
			}

			x := Interval{tile_edge(g, .x, i, tiles_x, pitch), tile_edge(g, .x, i + 1, tiles_x,
				pitch)}
			if on && !x.empty() {
				if run.hi < run.lo {
					run = x
				} else if run.touches(x) {
					run = run.join(x)
				} else {
					push_rect(mut out, run, y)
					run = x
				}
			} else if run.hi >= run.lo {
				push_rect(mut out, run, y)
				run = Interval{0, -1}
			}
		}
		if run.hi >= run.lo {
			push_rect(mut out, run, y)
		}
	}
}

fn tile_edge(g &Geometry, axis Axis, index int, count int, pitch int) int {
	return g.snap_edge(axis, g.center(axis) + (f32(index) - f32(count) / 2) * f32(pitch))
}

const braille_slots = [[0, 0], [0, 1], [0, 2], [1, 0], [1, 1],
	[1, 2], [0, 3], [1, 3]]

fn build_braille(code u32, g &Geometry, mut out []ShapeRect) {
	bits := int(code - 0x2800)
	a, b := f32(g.w) / 2, f32(g.h) / 4
	mut diameter := f32(rnd((if a < b { a } else { b }) * 0.6))
	if diameter < 1 {
		diameter = 1
	}
	half := diameter / 2
	for bit, slot in braille_slots {
		if bits & (1 << bit) == 0 {
			continue
		}
		cx := g.cx + (f32(slot[0]) - 0.5) * f32(g.w) / 2
		cy := g.cy + (f32(slot[1]) - 1.5) * f32(g.h) / 4
		push_rect(mut out, snap_interval(cx, half, g.w, .nearest), snap_interval(cy, half, g.h,
			.nearest))
	}
}

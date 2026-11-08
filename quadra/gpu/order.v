// Overlap layers: an instance must stay above everything it overlaps that was painted
// before it, and nothing else. Assigning layer = 1 + max(layer of earlier overlapping
// instances) and stable-sorting by (layer, page key) groups same-page instances while
// keeping every overlapping pair in paint order. The segments of one stroke (stroke.v) never
// stack on each other: the shader gives every pixel to one of them, so their order is free.
module gpu

import quadra.prim

const cell_size = f32(64)

// SpatialHash indexes instance bounds in a uniform grid; reused across frames.
pub struct SpatialHash {
mut:
	cells map[u64][]int
}

fn (mut h SpatialHash) clear() {
	for _, mut v in h.cells {
		v.clear()
	}
}

fn cell_range(r prim.Rect) (int, int, int, int) {
	return int(r.x / cell_size), int(r.y / cell_size), int((r.x + r.width) / cell_size), int((r.y +
		r.height) / cell_size)
}

fn key(cx int, cy int) u64 {
	return (u64(u32(cx)) << 32) | u64(u32(cy))
}

struct Ranked {
	layer int
	page  int
	index int
}

pub fn reorder_by_layers(mut instances []Instance, xforms []Rigid, mut h SpatialHash) {
	h.clear()
	mut layers := []int{len: instances.len}
	mut rects := []prim.Rect{cap: instances.len}
	mut run := 0 // first segment of the current stroke
	for i, inst in instances {
		cont := inst.kind == u32(Kind.line) && inst.flags & seg_cont != 0
		if !cont {
			run = i
		}
		r := inst.screen_rect(xforms)
		rects << r
		x0, y0, x1, y1 := cell_range(r)
		mut layer := 0
		for cy in y0 .. y1 + 1 {
			for cx in x0 .. x1 + 1 {
				for j in h.cells[key(cx, cy)] {
					if cont && j >= run {
						continue
					}
					if layers[j] + 1 > layer && rects[j].intersects(r) {
						layer = layers[j] + 1
					}
				}
			}
		}
		layers[i] = layer
		for cy in y0 .. y1 + 1 {
			for cx in x0 .. x1 + 1 {
				h.cells[key(cx, cy)] << i
			}
		}
	}
	mut ranked := []Ranked{cap: instances.len}
	for i, inst in instances {
		page := if inst.kind == u32(Kind.mono) {
			int(inst.page) * 2 + 1
		} else if inst.kind == u32(Kind.poly) {
			int(inst.page) * 2 + 2
		} else {
			0
		}
		ranked << Ranked{layers[i], page, i}
	}
	ranked.sort_with_compare(fn (a &Ranked, b &Ranked) int {
		if a.layer != b.layer {
			return a.layer - b.layer
		}
		if a.page != b.page {
			return a.page - b.page
		}
		return a.index - b.index
	})
	old := instances.clone()
	for i, r in ranked {
		instances[i] = old[r.index]
	}
}

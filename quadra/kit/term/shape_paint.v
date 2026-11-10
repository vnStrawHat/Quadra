// Painting the shapes of a row: rects coalesced across touching cells, one quad per run.
module term

import quadra.core
import quadra.gpu
import quadra.prim
import quadra.text

// ShapeQuad is a device-pixel rect from the grid's left edge with its final color.
struct ShapeQuad {
mut:
	x     int
	y     int
	w     int
	h     int
	color prim.Color
}

// push_shape adds a shape cell to the row: a rect that continues the previous column's
// (same color, same height, touching) extends it instead of becoming a new quad.
fn (g &Grid) push_shape(ch u32, idx int, col int, fg prim.Color, bold bool) {
	mut t := unsafe { g.t }
	touching := t.last_shape == col - 1
	t.last_shape = col
	if !touching {
		t.open_a.clear()
	}
	t.open_b.clear()
	dw, dh := t.cell_px[0], t.cell_px[1]
	weight := if bold { f32(int_min(t.b.weight + 300, 900)) } else { f32(t.b.weight) }
	offset := col * dw
	rects := if bold {
		t.shapes_bold.rects_of(idx, ch, dw, dh, weight)
	} else {
		t.shapes.rects_of(idx, ch, dw, dh, weight)
	}
	for r in rects {
		x := r.x + offset
		color := fg.alpha(fg.a * r.alpha)
		mut extended := -1
		for i in t.open_a {
			q := t.shape_row[i]
			if q.color == color && q.y == r.y && q.h == r.h && q.x + q.w == x {
				extended = i
				break
			}
		}
		if extended >= 0 {
			t.shape_row[extended].w += r.w
			t.open_b << extended
		} else {
			t.open_b << t.shape_row.len
			t.shape_row << ShapeQuad{x, r.y, r.w, r.h, color}
		}
	}
	t.open_a, t.open_b = t.open_b, t.open_a
}

// flush_shapes paints the row's shapes as quads.
fn (g &Grid) flush_shapes(y f32, mut cx core.PaintCx) {
	mut t := unsafe { g.t }
	if t.shape_row.len == 0 {
		return
	}
	s := cx.scale
	for q in t.shape_row {
		cx.quad(gpu.Quad{
			bounds:     prim.rect(t.bounds.x + f32(q.x) / s, y + f32(q.y) / s, f32(q.w) / s,
				f32(q.h) / s)
			background: q.color
		})
	}
	t.shape_row.clear()
}

// apply_features tells the text system the font features this terminal wants; only when they
// changed: re-shaping everything on every frame would be ruinous.
fn (mut t Terminal) apply_features(family string, mut ts text.TextSystem) {
	mut list := []text.FontFeature{cap: t.b.features.len + 1}
	if !t.b.ligatures {
		list << text.FontFeature{'calt', 0}
	}
	for f in t.b.features {
		list = list.filter(it.tag != f)
		list << text.FontFeature{f, 1}
	}
	sig := list.map('${it.tag}${it.value}').join(',')
	if sig != t.feature_sig {
		t.feature_sig = sig
		ts.font_features(family, list)
	}
}

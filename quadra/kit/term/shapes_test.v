// Cell-exact shape geometry and its cache (specs/17).
module term

import quadra.prim

fn rects(code u32, w int, h int, weight f32) []ShapeRect {
	mut out := []ShapeRect{}
	shape_quads(code, w, h, weight, mut out)
	return out
}

fn test_a_horizontal_line_spans_the_whole_cell_and_a_cross_is_the_union() {
	h := rects(0x2500, 8, 16, 400)
	assert h.len == 1 && h[0].x == 0 && h[0].w == 8 && h[0].alpha == 1
	v := rects(0x2502, 8, 16, 400)
	assert v.len == 1 && v[0].y == 0 && v[0].h == 16
	assert h[0].h == v[0].w // equally thick
	cross := rects(0x253c, 8, 16, 400)
	mut hit := [8 * 16]bool{}
	for r in cross {
		for y in r.y .. r.y + r.h {
			for x in r.x .. r.x + r.w {
				hit[y * 8 + x] = true
			}
		}
	}
	for y in 0 .. 16 {
		for x in 0 .. 8 {
			on_h := y >= h[0].y && y < h[0].y + h[0].h
			on_v := x >= v[0].x && x < v[0].x + v[0].w
			assert hit[y * 8 + x] == (on_h || on_v)
		}
	}
}

fn test_a_bold_cell_draws_heavier_strokes() {
	normal := rects(0x2500, 16, 32, 400)
	heavy := rects(0x2500, 16, 32, 700)
	assert heavy[0].h > normal[0].h
	assert rects(0x2501, 16, 32, 400)[0].h > normal[0].h // heavy horizontal
}

fn test_blocks_and_the_full_block() {
	full := rects(0x2588, 8, 16, 400)
	assert full.len == 1 && full[0].x == 0 && full[0].y == 0 && full[0].w == 8 && full[0].h == 16
	assert rects(0x2580, 8, 16, 400)[0].h == 8 // upper half
	assert rects(0x258f, 8, 16, 400)[0].w == 1 // one eighth left block, never empty
	assert rects(0x2590, 8, 16, 400)[0].x == 4 // right half
	q := rects(0x259a, 8, 16, 400)
	assert q.len == 2 && q[0].x == 0 && q[1].x == 4 && q[0].y == 0 && q[1].y == 8
	assert rects(0x25ac, 8, 16, 400).len == 1 // black rectangle
}

fn test_shades_are_tiles() {
	assert rects(0x2591, 9, 18, 400).len > 1
	mut dark, mut light := 0, 0
	for r in rects(0x2593, 9, 18, 400) {
		dark += r.w * r.h
	}
	for r in rects(0x2591, 9, 18, 400) {
		light += r.w * r.h
	}
	assert dark > light
}

fn test_braille_dots_are_cells() {
	assert rects(0x2800, 8, 16, 400).len == 0 // blank, and not font-drawn
	assert rects(0x28ff, 8, 16, 400).len == 8
	dot := rects(0x2801, 8, 16, 400) // the top-left dot
	assert dot.len == 1 && dot[0].x < 4 && dot[0].y < 4
}

fn test_curves_are_covered_and_mirrored() {
	// An odd width: the stroke is centred, so U+256E is U+256D reflected exactly in x.
	mut a := rects(0x256d, 9, 16, 400)
	assert a.len > 0
	mut b := rects(0x256e, 9, 16, 400)
	assert coverage(a, 9) == coverage(b, 9).map(reverse_row(it, 9))
	assert rects(0xe0b0, 8, 16, 400).len > 0 // powerline triangle
	assert rects(0xe0ba, 8, 16, 400).len > 0
}

fn coverage(rs []ShapeRect, w int) []string {
	mut rows := []string{len: 16, init: '.'.repeat(w)}
	for r in rs {
		for y in r.y .. r.y + r.h {
			mut b := rows[y].bytes()
			for x in r.x .. r.x + r.w {
				b[x] = `0` + u8(r.alpha * 9)
			}
			rows[y] = b.bytestr()
		}
	}
	return rows
}

fn reverse_row(s string, _ int) string {
	return s.reverse()
}

fn test_shape_index_is_dense() {
	mut seen := map[int]bool{}
	for c in [u32(0x2500), 0x259f, 0x25ac, 0x2800, 0x28ff, 0xe0b0, 0xe0bf] {
		i := shape_index(c)
		assert i >= 0 && i < shape_count && i !in seen
		seen[i] = true
	}
	assert shape_index(0x2200) == -1 && shape_index(0x25ab) == -1 && shape_index(0xe0c0) == -1
}

fn test_rects_are_cached_per_size_and_weight() {
	mut c := ShapeCache{}
	a := c.rects_of(shape_index(0x2500), 0x2500, 8, 16, 400)
	assert a.len == 1
	assert c.done[shape_index(0x2500)]
	b := c.rects_of(shape_index(0x2500), 0x2500, 10, 20, 400)
	assert b[0].w == 10
}

fn test_touching_cells_coalesce_into_one_quad() {
	mut t := &Terminal{
		screen: new_screen(10, 2)
	}
	t.cell_px = [8, 16]!
	g := Grid{
		t: t
	}
	fg := prim.rgb(0xffffff)
	for x in 0 .. 5 {
		g.push_shape(0x2500, shape_index(0x2500), x, fg, false)
	}
	assert t.shape_row.len == 1 && t.shape_row[0].w == 40
	g.push_shape(0x2500, shape_index(0x2500), 7, fg, false) // a gap breaks the run
	assert t.shape_row.len == 2
	t.shape_row.clear()
	// Alternating bands never coalesce.
	g.push_shape(0x2588, shape_index(0x2588), 0, fg, false)
	g.push_shape(0x2591, shape_index(0x2591), 1, fg, false)
	assert t.shape_row.len > 2
}

fn test_a_box_line_under_bold_differs_by_stroke() {
	mut t := &Terminal{
		screen: new_screen(4, 1)
	}
	t.cell_px = [16, 32]!
	g := Grid{
		t: t
	}
	fg := prim.rgb(0xffffff)
	g.push_shape(0x2500, shape_index(0x2500), 0, fg, false)
	g.push_shape(0x2500, shape_index(0x2500), 1, fg, true)
	assert t.shape_row.len == 2 // a bold cell never extends a plain neighbour
}

fn test_contrast_reaches_the_minimum() {
	bg := prim.rgb(0x101010)
	fg := prim.rgb(0x222222)
	out := ensure_contrast(fg, bg, 4.5)
	assert contrast_ratio(out, bg) >= 4.5
	assert ensure_contrast(fg, bg, 1) == fg // 1 is off
	red := ensure_contrast(prim.rgb(0x500000), prim.rgb(0x000000), 4.5)
	assert red.r > red.g && red.g == red.b // the hue is kept
}

fn test_text_contrast_spares_exact_colors_and_decorations() {
	mut t := &Terminal{
		screen: new_screen(4, 1)
	}
	t.b.min_contrast = 4.5
	fg, bg := prim.rgb(0x222222), prim.rgb(0x111111)
	pushed := t.text_contrast(Cell{ ch: u32(`x`) }, fg, bg)
	assert contrast_ratio(pushed, bg) >= 4.5
	assert t.text_contrast(Cell{ ch: u32(`x`), fg: palette(200) }, fg, bg) == fg // palette 16+
	assert t.text_contrast(Cell{ ch: u32(`x`), fg: rgb_color(1, 2, 3) }, fg, bg) == fg
	assert t.text_contrast(Cell{ ch: 0x2500 }, fg, bg) == fg // decorative
	t.b.min_contrast = 0
	assert t.text_contrast(Cell{ ch: u32(`x`) }, fg, bg) == fg
}

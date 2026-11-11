// Images in the grid (specs/17): a graphic is RGBA pixels, each row it crosses holds an
// ImageRef in Row.extra and moves with the text; painting draws the band of its row.
module term

import quadra.core
import quadra.gpu
import quadra.prim

// Graphic is a decoded image. It owns its pixels (C heap, PixelBuf) until the view hands them
// to the atlas upload; they are freed when the graphic is evicted, swept or dropped.
@[heap]
pub struct Graphic {
pub:
	id     u32
	width  int
	height int
	cols   int
	rows   int
mut:
	pixels PixelBuf
}

// ImageRef: a row shows image `id`, the part `row` rows down its height, from column col.
pub struct ImageRef {
pub:
	id   u32
	col  int
	row  int
	cols int
}

const max_placements = 256
const max_gpu_images = 64
const max_gpu_bytes = 64 << 20
const max_image_bytes = 64 << 20

fn cells_for(px int, cell int) int {
	return (px + cell - 1) / cell
}

// place_sixel puts a decoded image at the cursor and moves the cursor past its rows. It takes
// the image's pixels; the caller frees the decoder.
fn (mut s Screen) place_sixel(mut img Sixel) {
	if img.img_w == 0 || img.img_h == 0 || s.images_off {
		return
	}
	cursor_rows := img.band_px / s.cell_h
	cols := clamp(cells_for(img.img_w, s.cell_w), 0, s.cols - s.x)
	rows := clamp(cells_for(img.img_h, s.cell_h), 1, 1 << 15)
	s.next_graphic++
	g := &Graphic{
		id:     s.next_graphic
		width:  img.img_w
		height: img.img_h
		cols:   cols
		rows:   rows
		pixels: img.rgba.take()
	}
	s.graphics << g
	s.evict_graphics()
	col := s.x
	limit := if rows > cursor_rows + 1 { rows } else { cursor_rows + 1 }
	for row in 0 .. limit {
		y := if row <= cursor_rows { s.y } else { s.y + row - cursor_rows }
		if row < rows && y < s.rows {
			mut e := s.lines[y].extra_mut()
			e.images << ImageRef{g.id, col, row, cols}
			s.touch(y)
		}
		if row < cursor_rows {
			s.linefeed()
		}
	}
	s.wrap_pending = false
}

fn (mut s Screen) evict_graphics() {
	mut bytes := 0
	for g in s.graphics {
		bytes += g.width * g.height * 4
	}
	for s.graphics.len > 1 && (s.graphics.len > max_placements || bytes > max_image_bytes) {
		bytes -= s.graphics[0].width * s.graphics[0].height * 4
		mut old := s.graphics[0]
		old.pixels.free()
		s.graphics.delete(0)
	}
}

// drop_images removes this row's references that lie wholly inside columns a..b.
fn (mut r Row) drop_images(a int, b int) {
	if r.extra.images.len > 0 {
		r.extra.images = r.extra.images.filter(it.col < a || it.col + it.cols > b)
	}
}

// sweep_graphics forgets graphics no row refers to any more: rows are dropped, reused and
// trimmed in too many places to count references.
fn (mut s Screen) sweep_graphics() {
	mut live := map[u32]bool{}
	for r in s.lines {
		collect_images(r.extra, mut live)
	}
	for r in s.main {
		collect_images(r.extra, mut live)
	}
	for i in 0 .. s.scrollback.len() {
		collect_images(s.scrollback.at(i).extra, mut live)
	}
	for mut g in s.graphics {
		if !live[g.id] {
			g.pixels.free()
		}
	}
	s.graphics = s.graphics.filter(live[it.id])
}

// free_graphics drops every graphic and frees its pixels (reset, tests).
pub fn (mut s Screen) free_graphics() {
	for mut g in s.graphics {
		g.pixels.free()
	}
	s.graphics = []
}

// close_images frees the pixels not uploaded yet and drops images that arrive later
// (Terminal.close): an uploaded image keeps showing from its atlas tile.
fn (mut s Screen) close_images() {
	for mut g in s.graphics {
		g.pixels.free()
	}
	s.images_off = true
}

fn collect_images(e &RowExtra, mut live map[u32]bool) {
	if e != unsafe { nil } {
		for im in e.images {
			live[im.id] = true
		}
	}
}

// graphic returns a live graphic by id (few of them: a linear scan).
pub fn (s &Screen) graphic(id u32) ?&Graphic {
	for g in s.graphics {
		if g.id == id {
			return g
		}
	}
	return none
}

// cell_px tells the screen the device size of a cell (image placement, CSI 14 t).
pub fn (mut s Screen) cell_px(w int, h int) {
	if w > 0 && h > 0 {
		s.cell_w, s.cell_h = w, h
	}
}

// image_entry is the atlas tile of a graphic, uploading it (premultiplied RGBA) the first
// time. The upload takes the pixels (the window frees them once the device has copied them);
// an image evicted later has none left and paints nothing.
fn (mut t Terminal) image_entry(id u32, mut g Graphic, mut cx core.PaintCx) ?&core.ImageEntry {
	if e := t.gfx[id] {
		return e
	}
	g.fit_atlas()
	w, h, px := g.pixels.w, g.pixels.h, g.pixels.bytes()
	if px.len == 0 {
		return none
	}
	mut app := cx.app
	tile := app.text.poly.insert_owned(w, h, px) or {
		g.pixels.free()
		return none
	}
	mut e := &core.ImageEntry{}
	e.tile, e.width, e.height, e.status = tile, w, h, core.ImageStatus.ready
	_ = g.pixels.take() // the upload owns the block now
	t.gfx[id] = e
	t.evict_gfx()
	return e
}

// evict_gfx keeps the uploaded images within the limits, oldest out (map order is insertion
// order). An evicted image paints nothing; earlier sprites of this frame may still show its
// tile, so the tile goes back to the atlas at the next paint.
fn (mut t Terminal) evict_gfx() {
	mut bytes := 0
	for _, e in t.gfx {
		bytes += e.width * e.height * 4
	}
	for id in t.gfx.keys() {
		if t.gfx.len <= max_gpu_images && bytes <= max_gpu_bytes {
			break
		}
		e := t.gfx[id] or { continue }
		bytes -= e.width * e.height * 4
		t.gfx.delete(id)
		t.gfx_dead << e
	}
}

// fit_atlas scales an image past the atlas page down, nearest neighbour, into a new buffer;
// the original is freed at once.
fn (mut g Graphic) fit_atlas() {
	max := 2047
	pw, ph := g.pixels.w, g.pixels.h
	if pw <= max && ph <= max {
		return
	}
	bigger := if pw > ph { pw } else { ph }
	scaled := g.pixels.scaled(pw * max / bigger, ph * max / bigger)
	g.pixels.free()
	g.pixels = scaled
}

// sweep_gfx returns to the atlas the tiles of evicted images and of dead graphics.
fn (mut t Terminal) sweep_gfx(mut cx core.PaintCx) {
	mut app := cx.app
	for e in t.gfx_dead {
		app.text.poly.release(e.tile)
	}
	t.gfx_dead.clear()
	for id, e in t.gfx {
		if t.screen.graphic(id) == none {
			t.gfx.delete(id)
			app.text.poly.release(e.tile)
		}
	}
}

// with_release ties a new terminal to its view: dropping the view ends the session and gives
// back what lives outside the collector's heap.
fn with_release(mut cx core.Cx, v core.View[Terminal]) core.View[Terminal] {
	cx.on_release(v.model(), cx.peek(v.model()))
	return v
}

// release is core.Releasable: close, and the atlas tiles.
pub fn (mut t Terminal) release(mut app core.App) {
	t.close()
	for _, e in t.gfx {
		app.text.poly.release(e.tile)
	}
	for e in t.gfx_dead {
		app.text.poly.release(e.tile)
	}
	t.gfx = map[u32]&core.ImageEntry{}
	t.gfx_dead = []
}

// paint_images draws one row's bands of the images that cross it, in cell order.
fn (g &Grid) paint_images(row Row, y f32, mut cx core.PaintCx) {
	mut t := g.t
	for r in row.extra.images {
		mut gr := t.screen.graphic(r.id) or { continue }
		e := t.image_entry(r.id, mut gr, mut cx) or { continue }
		k_y := t.cell.height / f32(t.screen.cell_h)
		// Pixel rows of the image in this cell row.
		a := r.row * t.screen.cell_h
		mut b := a + t.screen.cell_h
		if b > gr.height {
			b = gr.height
		}
		if a >= b {
			continue
		}
		v0 := e.tile.v0 + (e.tile.v1 - e.tile.v0) * f32(a) / f32(gr.height)
		v1 := e.tile.v0 + (e.tile.v1 - e.tile.v0) * f32(b) / f32(gr.height)
		tile := gpu.AtlasTile{
			...e.tile
			v0: v0
			v1: v1
		}
		x := t.bounds.x + f32(r.col) * t.cell.width
		w := f32(gr.width) * t.cell.width / f32(t.screen.cell_w)
		cx.sprite(prim.rect(x, y, w, f32(b - a) * k_y), &core.ImageEntry{
			...e
			tile: tile
		}, prim.white)
	}
}

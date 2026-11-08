// GPU strokes (gpu/stroke.v): segment neighbours, exact antialiasing, joints covered once,
// canvas strokes and the spinner without atlas growth.
module main

import os
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.layout
import quadra.testing
import quadra.ui
import quadra.vector

fn test_segments_know_their_neighbours() {
	mut s := gpu.Scene{}
	s.stroke([prim.pt(0, 0), prim.pt(10, 0), prim.pt(10, 10)], 2, prim.black, false, gpu.ContentMask{})
	assert s.instances.len == 2
	assert s.instances[0].flags == gpu.seg_next
	assert s.instances[1].flags == gpu.seg_prev | gpu.seg_cont
	assert s.instances[1].uv[0] == 0 && s.instances[0].uv[2] == 10 && s.instances[0].uv[3] == 10
	// A closed square, with or without the first point repeated: four segments in a ring.
	sq := [prim.pt(0, 0), prim.pt(10, 0), prim.pt(10, 10), prim.pt(0, 10)]
	for pts in [sq, [sq[0], sq[1], sq[2], sq[3], sq[0]]] {
		s.clear()
		s.stroke(pts, 2, prim.black, true, gpu.ContentMask{})
		assert s.instances.len == 4
		last := s.instances[3]
		assert last.radii == [f32(0), 10, 0, 0]! // (0,10) -> (0,0)
		assert last.uv == [f32(10), 10, 10, 0]! // prev starts at (10,10), next ends at (10,0)
		assert s.instances[0].uv[0] == 0 && s.instances[0].uv[1] == 10 // prev of the first
		assert s.instances.all(it.flags & (gpu.seg_prev | gpu.seg_next) == 3)
	}
}

const half_black = prim.Color{0, 0, 0, 0.5}

struct Strokes {}

@[markused]
pub fn (mut v Strokes) render(mut cx core.ViewCx[Strokes]) core.Element {
	return ui.div().flex_col().size_full().bg(prim.white).child(ui.canvas(fn (mut p ui.Painter, _ prim.Size) {
		p.line(10, 20.5, 90, 20.5, 2, prim.black) // horizontal, on a pixel row's center
		p.line(10, 40, 90, 120, 2, prim.black) // 45 degrees
		mut vv := vector.new_path() // a translucent V with a right-angle joint at (150, 100)
		vv.move_to(110, 60)
		vv.line_to(150, 100)
		vv.line_to(190, 60)
		p.stroke_path(vv, 4, half_black)
	}).size(200, 140)).child(kit.spinner().size(24))
}

fn px(rgba []u8, w int, x int, y int) int {
	return rgba[(y * w + x) * 4] // red channel: gray level on white
}

fn test_strokes_on_the_gpu() {
	mut h := testing.new_pixel_harness()!
	kit.install(mut h.app)!
	h.open(Strokes{}, width: 200, height: 200)!
	img := h.rgba()!
	os.mkdir_all(os.join_path(os.dir(@FILE), '..', '.tmp', 'out'))!
	h.save_png(os.join_path(os.dir(@FILE), '..', '.tmp', 'out', 'strokes.png'))!
	// 2 px wide around y = 20.5 covers 19.5..21.5: row 20 fully, rows 19 and 21 half each.
	assert px(img, 200, 50, 20) < 10
	assert px(img, 200, 186, 64) < 150 // the V's right arm is all there
	assert px(img, 200, 50, 19) > 100 && px(img, 200, 50, 19) < 155
	assert px(img, 200, 50, 21) > 100 && px(img, 200, 50, 21) < 155
	assert px(img, 200, 50, 24) > 250
	// The diagonal: dark on its center, symmetric across it, white 3 px away.
	assert px(img, 200, 50, 80) < 40
	assert (px(img, 200, 51, 80) - px(img, 200, 50, 81)) in [-2, -1, 0, 1, 2]
	assert px(img, 200, 53, 80) > 250
	// The translucent joint is covered once: half gray like the arms, not darker.
	arm := px(img, 200, 130, 80)
	joint := px(img, 200, 150, 99)
	assert arm > 115 && arm < 140
	assert joint > 115 && joint < 140
}

fn test_spinner_animates_without_atlas_growth() {
	mut h := testing.new_pixel_harness()!
	kit.install(mut h.app)!
	h.open(Strokes{}, width: 200, height: 200)!
	before := h.app.images.entries.len
	for _ in 0 .. 40 {
		h.advance(16) // two turns and a bit, at every angle
	}
	assert h.app.images.entries.len == before
}

// Wavy paints one wavy underline (gpu.LinePrim.wavy) across its box.
struct Wavy {
mut:
	node layout.NodeId
}

fn (mut w Wavy) request_layout(mut cx core.LayoutCx) layout.NodeId {
	w.node = cx.tree.node(layout.Style{
		width:  layout.px(64)
		height: layout.px(8)
	}, [])
	return w.node
}

fn (mut w Wavy) prepaint(_ prim.Rect, mut _ core.PrepaintCx) {}

fn (mut w Wavy) paint(bounds prim.Rect, mut cx core.PaintCx) {
	cx.line(gpu.LinePrim{
		bounds:    bounds
		color:     prim.black
		thickness: 1.5
		wavy:      true
	})
}

struct WavyView {}

@[markused]
pub fn (mut v WavyView) render(mut cx core.ViewCx[WavyView]) core.Element {
	return ui.div().size_full().bg(prim.white).p(8).child(&Wavy{})
}

// ink is how much darkness column x holds (rows 0..24).
fn ink(img []u8, x int) int {
	mut sum := 0
	for y in 0 .. 24 {
		sum += 255 - px(img, 80, x, y)
	}
	return sum
}

fn test_wavy_underline_keeps_its_width_on_the_slopes() {
	mut h := testing.new_pixel_harness()!
	h.open(WavyView{}, width: 80, height: 24)!
	img := h.rgba()!
	// An even width w across the wave puts w * sqrt(1 + slope²) of ink in a column: about
	// 2.7 times more where it is steepest (slope 2.55) than at a crest (about 2 here: the
	// distance is first order and the edges add 1 px of antialiasing). Measured straight
	// down, as before, every column held the same ink and the slopes looked thin.
	cols := []int{len: 32, init: ink(img, 8 + index)}
	mut lo, mut hi := cols[0], cols[0]
	for c in cols {
		lo, hi = if c < lo { c } else { lo }, if c > hi { c } else { hi }
	}
	assert lo > 200 // every column is drawn
	assert f64(hi) / f64(lo) > 1.6
}

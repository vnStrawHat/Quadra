// Rotation (gpu.Rigid, scene xforms) and animation timelines (ui/div_timeline.v).
module main

import math
import os
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.testing
import quadra.ui

const red = prim.Color{1, 0, 0, 1}
const blue = prim.Color{0, 0, 1, 1}

fn near(a f32, b f32) bool {
	return math.abs(a - b) < 0.01
}

fn test_rigid_rotates_composes_and_inverts() {
	r := gpu.rotation_about(90, 10, 10)
	x, y := r.apply(20, 10)
	assert near(x, 10) && near(y, 20) // clockwise on screen (y down)
	both := r.then(gpu.rotation_about(-90, 10, 10))
	bx, by := both.apply(3, 7)
	assert near(bx, 3) && near(by, 7)
	ix, iy := r.inverse().apply(x, y)
	assert near(ix, 20) && near(iy, 10)
	b := gpu.rotation_about(45, 0, 0).bbox(prim.rect(-1, -1, 2, 2))
	assert near(b.width, f32(2 * math.sqrt2))
}

struct Turned {}

@[markused]
pub fn (mut t Turned) render(mut cx core.ViewCx[Turned]) core.Element {
	card :=
		ui.div().test_id('card').absolute().left(100).top(100).size(40).rotate(90).overflow_hidden().child(ui.div().absolute().left(0).top(0).size(10).bg(red))
	return ui.div().size_full().child(card)
}

fn first_quad(h &testing.Harness, c prim.Color) gpu.Instance {
	for inst in h.scene().instances {
		if inst.kind == u32(gpu.Kind.quad) && inst.color[0] == c.r && inst.color[2] == c.b {
			return inst
		}
	}
	panic('no quad')
}

fn test_rotation_turns_primitives_on_the_gpu_and_keeps_hit_tests() {
	mut h := testing.new_harness(width: 400, height: 400)!
	h.open(Turned{}, width: 400, height: 400)!
	q := first_quad(h, red)
	assert q.bounds[0] == 100 && q.bounds[1] == 100 // own space: unrotated
	own := int(q.xform & 0xffff)
	assert own != 0
	assert q.xform >> 16 == u32(own) // the card's clip is in the same space
	// About the center (120, 120), clockwise: the top-left corner goes to the top right.
	x, y := h.scene().xforms[own].apply(100, 100)
	assert near(x, 140) && near(y, 100)
	assert q.screen_rect(h.scene().xforms).x > 129 // the bounding box used for overlap
	card := h.find('card') or { panic('no card') }
	assert card.x == 100 && card.y == 100
}

struct Timed {
	m ui.Motion
}

@[markused]
pub fn (mut t Timed) render(mut cx core.ViewCx[Timed]) core.Element {
	return ui.div().size_full().child(ui.div().absolute().left(0).top(0).size(20).bg(red).animate(t.m))
}

fn red_at(mut h testing.Harness, ms int) gpu.Instance {
	h.advance(ms)
	h.frame()
	return first_quad(h, red)
}

// alpha_at is the red quad's alpha after ms more (0 when it is not drawn: transparent).
fn alpha_at(mut h testing.Harness, ms int) f32 {
	h.advance(ms)
	h.frame()
	for inst in h.scene().instances {
		if inst.kind == u32(gpu.Kind.quad) && inst.color[0] == 1 {
			return inst.color[3]
		}
	}
	return 0
}

fn test_sequence_runs_steps_one_after_another_and_keeps_the_end() {
	m := ui.tween(100, ui.Patch{ opacity: 0 }) + ui.pause(50) + ui.tween(100, ui.Patch{
		opacity: 0.5
	})
	mut h := testing.new_harness(width: 100, height: 100)!
	h.open(Timed{m}, width: 100, height: 100)!
	assert near(first_quad(h, red).color[3], 1)
	assert near(alpha_at(mut h, 50), 0.5) // ease_in_out is half way at half time
	assert near(alpha_at(mut h, 80), 0) // in the pause
	assert near(alpha_at(mut h, 70), 0.25) // half way to 0.5
	assert near(alpha_at(mut h, 500), 0.5)
	assert !h.app.is_animating()
}

fn test_parallel_runs_together_and_repeat_alternates() {
	m := ui.parallel(ui.tween_with(100, .linear, ui.Patch{ translate: prim.Point{10, 0} }), ui.tween_with(200,
		.linear, ui.Patch{ opacity: 0 })).repeat(2).alternate()
	mut h := testing.new_harness(width: 100, height: 100)!
	h.open(Timed{m}, width: 100, height: 100)!
	q := red_at(mut h, 100)
	assert near(q.bounds[0], 10) && near(q.color[3], 0.5)
	back := red_at(mut h, 150) // second play, backwards: 50 ms before the end
	assert near(back.bounds[0], 10) && near(back.color[3], 0.25)
	done := red_at(mut h, 1000) // an even count of alternating plays ends at the start
	assert near(done.bounds[0], 0) && near(done.color[3], 1)
}

fn test_forever_keeps_going() {
	m := (ui.tween_with(100, .linear, ui.Patch{ rotate: 90 })).forever()
	mut h := testing.new_harness(width: 100, height: 100)!
	h.open(Timed{m}, width: 100, height: 100)!
	q := red_at(mut h, 1050)
	own := int(q.xform & 0xffff)
	assert own != 0
	r := h.scene().xforms[own]
	assert near(r.c, f32(math.cos(math.pi / 4))) // 45 degrees, half way through a play
}

struct Bars {}

@[markused]
pub fn (mut b Bars) render(mut cx core.ViewCx[Bars]) core.Element {
	bar := ui.div().absolute().left(100).top(100).w(60).h(10).bg(red).rotate(90)
	// A 40 px card turned 45 degrees clips a child that overflows it on every side.
	card :=
		ui.div().absolute().left(200).top(100).size(40).rotate(45).overflow_hidden().child(ui.div().absolute().left(-20).top(-20).size(80).bg(blue))
	return ui.div().size_full().bg(prim.white).child(bar).child(card).child(ui.div().absolute().left(20).top(200).rotate(30).child(ui.text('Turned')))
}

fn px_at(rgba []u8, w int, x int, y int) [3]u8 {
	i := (y * w + x) * 4
	return [rgba[i], rgba[i + 1], rgba[i + 2]]!
}

fn test_rotation_and_rotated_clips_on_the_gpu() {
	mut h := testing.new_pixel_harness()!
	h.open(Bars{}, width: 300, height: 300)!
	img := h.rgba()!
	os.mkdir_all(os.join_path(os.dir(@FILE), '..', '.tmp', 'out'))!
	h.save_png(os.join_path(os.dir(@FILE), '..', '.tmp', 'out', 'rotation.png'))!
	assert px_at(img, 300, 130, 80) == [u8(255), 0, 0]! // the bar now stands upright
	assert px_at(img, 300, 105, 105) == [u8(255), 255, 255]! // where it was laid out
	assert px_at(img, 300, 220, 120) == [u8(0), 0, 255]! // the card's center
	assert px_at(img, 300, 202, 102) == [u8(255), 255, 255]! // outside the turned card
	assert px_at(img, 300, 220, 93) == [u8(0), 0, 255]! // its top corner, past the layout box
}

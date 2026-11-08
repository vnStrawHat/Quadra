// Div transforms and transitions (ui/div_motion.v, core push_transform).
module main

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.testing
import quadra.ui

const red = prim.Color{1, 0, 0, 1}
const blue = prim.Color{0, 0, 1, 1}

struct Moved {
mut:
	k f32 = 2
}

@[markused]
pub fn (mut m Moved) render(mut cx core.ViewCx[Moved]) core.Element {
	box :=
		ui.div().absolute().left(100).top(100).size(40).translate(10, 0).scale(m.k).child(ui.div().absolute().left(0).top(0).size(10).bg(red))
	hover := ui.div().test_id('hov').absolute().left(0).top(200).size(50).bg(red).transition(100).hover(ui.Patch{
		background: blue
	})
	return ui.div().size_full().child(box).child(hover)
}

// quad_of finds the first quad of color c (device px equal logical px here, scale 1).
fn quad_of(h &testing.Harness, c prim.Color) ?gpu.Instance {
	for inst in h.scene().instances {
		if inst.kind == u32(gpu.Kind.quad) && inst.color[0] == c.r && inst.color[2] == c.b {
			return inst
		}
	}
	return none
}

fn test_scale_and_translate_move_children_and_keep_hit_tests() {
	mut h := testing.new_harness(width: 400, height: 400)!
	h.open(Moved{}, width: 400, height: 400)!
	q := quad_of(h, red) or { panic('no quad') }
	// Center (120, 120): the child at (100, 100) scales by 2 about it, then moves 10 right.
	assert q.bounds[0] == 90 && q.bounds[1] == 80
	assert q.bounds[2] == 20 && q.bounds[3] == 20
	hov := h.find('hov') or { panic('no hover box') }
	assert hov.x == 0 && hov.y == 200 // layout, so hit testing, does not move
}

fn test_hover_transition_eases_the_background() {
	mut h := testing.new_harness(width: 400, height: 400)!
	h.open(Moved{}, width: 400, height: 400)!
	mut first_red := true
	for inst in h.scene().instances {
		if inst.bounds[1] == 200 {
			first_red = inst.color[0] == 1
		}
	}
	assert first_red
	h.move_to(25, 225)
	h.advance(50)
	h.frame()
	mid := bg_at(h, 200)
	assert mid[0] > 0.05 && mid[0] < 0.95 // between red and blue
	assert math.abs(mid[0] + mid[2] - 1) < 0.01
	h.advance(100)
	h.frame()
	done := bg_at(h, 200)
	assert done[0] == 0 && done[2] == 1
}

fn bg_at(h &testing.Harness, y f32) [4]f32 {
	for inst in h.scene().instances {
		if inst.kind == u32(gpu.Kind.quad) && inst.bounds[1] == y {
			return inst.color
		}
	}
	panic('no quad at ${y}')
}

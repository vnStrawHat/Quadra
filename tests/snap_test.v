// Quads on the device pixel grid and per-axis clipping (specs/08, Device pixels, Clipping).
module main

import quadra.core
import quadra.prim
import quadra.gpu
import quadra.testing
import quadra.ui

const edge = prim.Color{1, 0, 0, 1}
const band = prim.Color{0, 0, 1, 1}

struct Snapped {}

@[markused]
pub fn (mut s Snapped) render(mut cx core.ViewCx[Snapped]) core.Element {
	// A bordered box with a ring at a fractional y, inside a vertical scroller as wide as it.
	field :=
		ui.div().absolute().left(0).top(10.4).w(100).h(32).border(1).border_color(edge).ring(3, band)
	scroller := ui.div().absolute().left(50).top(20).w(100).h(80).overflow_y_scroll().child(field)
	return ui.div().size_full().child(scroller)
}

fn quads_of(h &testing.Harness, c prim.Color) []gpu.Instance {
	return h.scene().instances.filter(it.kind == u32(gpu.Kind.quad) && it.color2[0] == c.r
		&& it.color2[2] == c.b)
}

fn whole(v f32) bool {
	return v == f32(int(v))
}

fn test_quads_snap_to_device_pixels_and_rings_are_even() {
	for s in [f32(1), 1.25, 1.5] {
		mut h := testing.new_harness(width: 300, height: 200)!
		h.open(Snapped{}, width: 300, height: 200)!
		h.plat.set_scale(h.win, s)
		h.frame()
		e := quads_of(h, edge)[0]
		r := quads_of(h, band)[0]
		for v in [e.bounds[0], e.bounds[1], e.bounds[2], e.bounds[3], r.bounds[0], r.bounds[1],
			r.bounds[2], r.bounds[3]] {
			assert whole(v), 'scale ${s}: ${e.bounds} ${r.bounds}'
		}
		assert e.border[0] == e.border[2] && e.border[1] == e.border[3] && e.border[0] >= 1
		// The band is one width on every side and ends where the box begins.
		w := r.border[0]
		assert r.border[1] == w && r.border[2] == w && r.border[3] == w && whole(w)
		assert r.bounds[0] + w == e.bounds[0] && r.bounds[1] + w == e.bounds[1]
		assert r.bounds[2] - 2 * w == e.bounds[2] && r.bounds[3] - 2 * w == e.bounds[3]
		// A vertical scroller clips its top and bottom only: the band shows at the sides.
		assert r.clip[0] <= r.bounds[0] && r.clip[0] + r.clip[2] >= r.bounds[0] + r.bounds[2]
	}
}

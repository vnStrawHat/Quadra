// The D3D11 device uploads only the patched instance range on hover frames; the image must
// be identical to a full paint (WARP renderer, same as the other pixel tests).
module main

import quadra.core
import quadra.prim
import quadra.testing
import quadra.ui

struct Tiles {}

@[markused]
pub fn (mut t Tiles) render(mut cx core.ViewCx[Tiles]) core.Element {
	mut d := ui.div().flex_row().wrap().gap(4).p(8).bg(prim.white)
	for i in 0 .. 40 {
		d.child(ui.div().test_id('t${i}').px(8).py(4).rounded(4).bg(prim.rgb(0x2563EB)).hover(ui.Patch{
			background: prim.rgb(0xDC2626)
		}).text_color(prim.white).child(ui.text('Tile ${i}')))
	}
	return d
}

fn test_patched_frame_matches_full_paint_on_the_gpu() {
	mut h := testing.new_pixel_harness()!
	h.open(Tiles{}, width: 600, height: 300)!
	w := h.window()
	for id in ['t5', 't22'] {
		r := h.find(id) or { panic(id) }
		patches := w.stats.patches
		h.move_to(r.x + 3, r.y + 3)
		assert w.stats.patches == patches + 1
		patched := h.rgba()!
		mut win := h.window()
		win.invalidate_paint()
		h.frame()
		assert h.rgba()! == patched, 'GPU image after a patch differs from a full paint (${id})'
	}
}

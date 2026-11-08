// Paint patches: a hover or press restyle repaints only the affected elements, and the
// patched scene is exactly what a full paint would draw.
module main

import quadra.core
import quadra.prim
import quadra.platform
import quadra.testing
import quadra.ui

struct Buttons {}

@[markused]
pub fn (mut b Buttons) render(mut cx core.ViewCx[Buttons]) core.Element {
	mut d := ui.div().flex_row().wrap().gap(4).p(8)
	for i in 0 .. 60 {
		mut btn := ui.div().test_id('b${i}').px(8).py(4).rounded(4).cursor(.pointer).hover(ui.Patch{
			background: prim.rgb(0x1D4ED8)
		}).active(ui.Patch{
			background: prim.rgb(0x1E3A8A)
		}).child(ui.text('Button ${i}'))
		if i % 2 == 0 {
			btn.bg(prim.rgb(0x2563EB)) // odd ones are ghost buttons: transparent until hovered
		}
		d.child(btn)
	}
	return d
}

fn full_paint_snapshot(mut h testing.Harness) string {
	mut w := h.window()
	w.invalidate_paint()
	h.frame()
	return h.snapshot()
}

fn test_hover_and_press_are_patched_like_a_full_paint() {
	mut h := testing.new_harness(width: 900, height: 400)!
	h.open(Buttons{})!
	w := h.window()
	for id in ['b3', 'b4', 'b17'] { // ghost and filled buttons
		r := h.find(id) or { panic(id) }
		paints := w.stats.paints
		patches := w.stats.patches
		h.move_to(r.x + 3, r.y + 3)
		assert w.stats.patches == patches + 1, 'hover over ${id} was not patched'
		assert w.stats.paints == paints, 'hover over ${id} repainted the window'
		patched := h.snapshot()
		assert patched == full_paint_snapshot(mut h), 'patched scene differs for ${id}'
	}
	r := h.find('b4') or { panic('') }
	h.move_to(r.x + 3, r.y + 3)
	before := w.stats.patches
	h.inject(platform.MouseEvent{
		action: .down
		button: .left
		x:      r.x + 3
		y:      r.y + 3
	})
	assert w.stats.patches > before // the press style is a patch too
	pressed := h.snapshot()
	assert pressed == full_paint_snapshot(mut h)
	h.inject(platform.MouseEvent{
		action: .up
		button: .left
		x:      r.x + 3
		y:      r.y + 3
	})
	assert h.snapshot() == full_paint_snapshot(mut h)
}

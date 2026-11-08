// ClearType on real pixels (WARP + DirectWrite): subpixel glyphs blend per channel through
// dual-source blending, and everything else still blends as premultiplied alpha.
module main

import quadra.core
import quadra.prim
import quadra.testing
import quadra.ui

struct Sample {}

@[markused]
pub fn (mut s Sample) render(mut cx core.ViewCx[Sample]) core.Element {
	return ui.div().flex_col().size_full().bg(prim.white).child(ui.div().p(8).child(ui.text('Hamburgefonstiv 0123').size(14).color(prim.black))).child(ui.div().w(40).h(20).bg(prim.rgba(0xFF000080)))
}

// tinted counts pixels of the text strip whose red and blue differ: color fringes.
fn tinted(cleartype bool) !(int, []u8) {
	mut h := testing.new_pixel_harness(cleartype: cleartype)!
	h.open(Sample{}, width: 200, height: 60)!
	px := h.rgba()!
	mut n := 0
	for y in 0 .. 30 {
		for x in 0 .. 200 {
			i := (y * 200 + x) * 4
			if (int(px[i]) - int(px[i + 2])) > 24 || (int(px[i + 2]) - int(px[i])) > 24 {
				n++
			}
		}
	}
	return n, px
}

fn test_cleartype_blends_per_subpixel() {
	n, px := tinted(true)!
	assert n > 20, 'ClearType text should have color fringes, got ${n} tinted pixels'
	// Black text over white only shows fringes if the destination is attenuated per channel.
	// A half-transparent red quad over white still blends as premultiplied alpha.
	i := (45 * 200 + 20) * 4
	assert px[i] > 250 && px[i + 1] > 120 && px[i + 1] < 136 && px[i + 2] > 120 && px[i + 2] < 136
}

fn test_grayscale_text_has_no_fringes() {
	n, _ := tinted(false)!
	assert n == 0
}

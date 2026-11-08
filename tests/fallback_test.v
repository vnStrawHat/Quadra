// Font fallback on DirectWrite: characters the requested font lacks come from a system font.
module main

import quadra.text
import quadra.text.dwrite

fn test_cjk_and_symbols_fall_back_to_a_system_font() {
	mut b := dwrite.new_backend()!
	mono := b.face(text.FontKey{'Cascadia Mono', 400, false})!
	latin := b.shape('abc', mono, 14)
	assert latin.len == 3 && latin.all(it.face == mono && it.id != 0)
	mixed := b.shape('a漢字b', mono, 14)
	assert mixed.len == 4
	assert mixed[0].face == mono && mixed[3].face == mono
	assert mixed[1].face != mono && mixed[1].face == mixed[2].face
	assert mixed.all(it.id != 0), 'a missing glyph (tofu): ${mixed.map(it.id)}'
	// Clusters still point at the right UTF-8 bytes, and pens keep advancing.
	assert mixed.map(it.cluster) == [0, 1, 4, 7]
	assert mixed[1].x > mixed[0].x && mixed[3].x > mixed[2].x
	bmp := b.raster(mixed[1].face, mixed[1].id, 14, 0)!
	assert bmp.width > 0 && bmp.height > 0
	// Vietnamese stays in the requested font.
	vi := b.shape('Tiếng Việt', mono, 14)
	assert vi.all(it.face == mono && it.id != 0)
}

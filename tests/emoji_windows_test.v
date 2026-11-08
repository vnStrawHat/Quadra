// Color emoji (text/dwrite/color_windows.c.v): an emoji from a color font rasterizes as
// a multi-color RGBA bitmap; a plain letter stays coverage.
module main

import quadra.text
import quadra.text.dwrite

fn test_emoji_rasterizes_in_color() {
	mut b := dwrite.new_backend()!
	ui := b.face(text.FontKey{ family: 'Segoe UI' })!
	shaped := b.shape('😀', ui, 32) // falls back to the emoji font
	assert shaped.len == 1
	bmp := b.raster(shaped[0].face, shaped[0].id, 32, 0)!
	assert bmp.color
	assert bmp.width > 16 && bmp.pixels.len == bmp.width * bmp.height * 4
	mut colors := map[u32]bool{}
	for i in 0 .. bmp.width * bmp.height {
		p := bmp.pixels[i * 4..i * 4 + 4]
		if p[3] == 255 {
			colors[(u32(p[0]) << 16) | (u32(p[1]) << 8) | u32(p[2])] = true
		}
	}
	assert colors.len >= 3 // face, eyes, mouth
	letter := b.shape('A', ui, 32)
	plain := b.raster(letter[0].face, letter[0].id, 32, 0)!
	assert !plain.color
}

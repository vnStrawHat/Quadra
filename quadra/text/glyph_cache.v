// Glyph tiles: rasterized on first use into the mono atlas, or the RGBA poly atlas for color
// and subpixel (ClearType) glyphs.
module text

import quadra.gpu

pub struct GlyphTile {
pub:
	tile     gpu.AtlasTile
	left     int
	top      int
	color    bool
	empty    bool
	subpixel bool
}

// subpixel_steps is the horizontal glyph-position quantization (1/4 px).
pub const subpixel_steps = 4

// glyph_tile returns the atlas tile for a glyph at a device px size and a subpixel
// phase in [0, subpixel_steps).
pub fn (mut ts TextSystem) glyph_tile(face FaceId, id u16, size_px f32, phase int) GlyphTile {
	key := (u64(face) << 48) | (u64(id) << 32) | (u64(u32(size_px * 16)) << 4) | u64(phase)
	if t := ts.tiles[key] {
		return t
	}
	bmp := ts.backend.raster(face, id, size_px, f32(phase) / subpixel_steps) or { GlyphBitmap{} }
	ts.stats.glyphs_rastered++
	mut t := GlyphTile{
		empty: true
	}
	if bmp.width > 0 && bmp.height > 0 {
		tile := if bmp.color || bmp.subpixel {
			ts.poly.insert(bmp.width, bmp.height, bmp.pixels) or { gpu.AtlasTile{} }
		} else {
			ts.mono.insert(bmp.width, bmp.height, bmp.pixels) or { gpu.AtlasTile{} }
		}
		t = GlyphTile{tile, bmp.left, bmp.top, bmp.color, tile.w == 0, bmp.subpixel}
	}
	ts.tiles[key] = t
	return t
}

// take_uploads collects pending atlas uploads for the GPU device.
pub fn (mut ts TextSystem) take_uploads() []gpu.Upload {
	mut out := ts.mono.take_uploads()
	out << ts.poly.take_uploads()
	return out
}

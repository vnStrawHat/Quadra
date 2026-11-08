// text.Backend on DirectWrite: font matching, script analysis, shaping and rasterization.
module dwrite

import quadra.platform.win
import quadra.text

@[heap]
pub struct DWriteBackend {
mut:
	f        &Factory = unsafe { nil }
	faces    []FontFace
	keys     []text.FontKey
	scratch  Scratch
	loc      &u16 = unsafe { nil } // locale as UTF-16, made once
	loc_of   string
	ascii    [2]ScriptAnalysis // the analyses of printable ASCII: [0] without letters, [1] with
	ascii_ok bool
	no_calt  []string // families shaped without contextual alternates
pub mut:
	locale string = 'en-us'
	// cleartype renders subpixel glyphs; it follows the Windows setting (apps may turn it
	// off, e.g. for windows that are not opaque).
	cleartype bool
	bgr       bool
}

pub fn new_backend() !&DWriteBackend {
	cleartype, bgr := win.font_smoothing()
	return &DWriteBackend{
		f:         new_factory()!
		cleartype: cleartype
		bgr:       bgr
	}
}

pub fn (b &DWriteBackend) default_family() string {
	return 'Segoe UI'
}

pub fn (mut b DWriteBackend) face(key text.FontKey) !text.FaceId {
	for i, k in b.keys {
		if k == key {
			return i
		}
	}
	ff := b.f.match_font(key.family, key.weight, if key.italic {
		font_style_italic
	} else {
		font_style_normal
	})!
	b.faces << FontFace{
		...ff
		no_calt: key.family in b.no_calt
	}
	b.keys << key
	return b.faces.len - 1
}

pub fn (b &DWriteBackend) metrics(face text.FaceId) text.FaceMetrics {
	m := b.faces[face].metrics
	upem := f32(m.design_units_per_em)
	return text.FaceMetrics{
		ascent:              f32(m.ascent) / upem
		descent:             f32(m.descent) / upem
		line_gap:            f32(m.line_gap) / upem
		underline_position:  -f32(m.underline_position) / upem
		underline_thickness: f32(m.underline_thickness) / upem
	}
}

pub fn (b &DWriteBackend) raster(face text.FaceId, glyph u16, size_px f32, subpixel_x f32) !text.GlyphBitmap {
	bmp := b.f.raster(&b.faces[face], glyph, size_px, subpixel_x, b.cleartype, b.bgr)!
	return text.GlyphBitmap{
		left:     bmp.left
		top:      bmp.top
		width:    bmp.width
		height:   bmp.height
		subpixel: bmp.subpixel
		color:    bmp.color
		pixels:   bmp.pixels
	}
}

pub fn (mut b DWriteBackend) contextual_off(family string) {
	b.no_calt << family
	for i, k in b.keys {
		if k.family == family {
			b.faces[i] = FontFace{
				...b.faces[i]
				no_calt: true
			}
		}
	}
}

pub fn (mut b DWriteBackend) load_fonts(files [][]u8) ![]string {
	return b.f.load_fonts(files)
}

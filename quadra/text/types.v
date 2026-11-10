// Text types and the per-OS backend interface (specs/04-text.md).
module text

import quadra.prim

pub type FaceId = int

pub struct FontKey {
pub:
	family string
	weight int = 400
	italic bool
}

pub fn (k FontKey) str() string {
	return '${k.family}|${k.weight}|${k.italic}'
}

// FaceMetrics are per em; multiply by the font size in px.
pub struct FaceMetrics {
pub:
	ascent              f32
	descent             f32
	line_gap            f32
	underline_position  f32 // below the baseline, positive down
	underline_thickness f32
}

// ShapedGlyph is one glyph of a shaped run. x/y are relative to the run's pen start;
// cluster is the UTF-8 byte offset (in the shaped text) of the first character it covers.
pub struct ShapedGlyph {
pub:
	face    FaceId
	id      u16
	x       f32
	y       f32
	advance f32
	cluster int
}

// GlyphBitmap is 8-bit coverage, RGBA when color is true, or per-subpixel RGBA coverage
// (r, g, b in screen order, a = their max) when subpixel is true; left/top are relative to
// the pen position on the baseline.
pub struct GlyphBitmap {
pub:
	left     int
	top      int
	width    int
	height   int
	color    bool
	subpixel bool
	pixels   []u8
}

pub interface Backend {
mut:
	default_family() string
	face(key FontKey) !FaceId
	metrics(face FaceId) FaceMetrics
	// shape shapes a whole run, choosing fallback faces for unsupported characters.
	shape(text string, face FaceId, size f32) []ShapedGlyph
	raster(face FaceId, glyph u16, size_px f32, subpixel_x f32) !GlyphBitmap
	load_fonts(files [][]u8) ![]string // registers bundled fonts, returns their family names
	// contextual_off shapes the family without contextual alternates (OpenType calt), for
	// interface faces whose alternates suit code, not labels. Call it before shaping.
	contextual_off(family string)
	// font_features sets the OpenType features the family is shaped with ([] = defaults).
	font_features(family string, features []FontFeature)
	// font_families lists the installed and loaded families, sorted ([] when it cannot).
	font_families() []string
}

// FontFeature is an OpenType feature setting: a 4-letter tag and its value (1 on, 0 off).
pub struct FontFeature {
pub:
	tag   string
	value u32
}

pub struct TextStyle {
pub:
	family      string // empty = backend default
	size        f32 = 14
	weight      int = 400
	italic      bool
	color       prim.Color = prim.black
	line_height f32        = 1.4 // multiple of size
}

pub fn (s TextStyle) key() FontKey {
	return FontKey{s.family, s.weight, s.italic}
}

pub fn (s TextStyle) line_px() f32 {
	return s.size * s.line_height
}

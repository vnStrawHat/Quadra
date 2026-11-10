// Deterministic text backend for headless tests: no OS, identical on every machine.
// Every grapheme advances 0.6 em; ascent 0.8 em, descent 0.2 em; glyphs rasterize as
// solid boxes so scene snapshots still see them.
module fake

import quadra.text

pub struct FakeBackend {
mut:
	families []string = ['Fake Sans']
	faces    []text.FontKey
}

pub fn new() &FakeBackend {
	return &FakeBackend{}
}

pub fn (b &FakeBackend) default_family() string {
	return 'Fake Sans'
}

pub fn (mut b FakeBackend) face(key text.FontKey) !text.FaceId {
	if key.family !in b.families {
		return error('unknown family ${key.family}')
	}
	for i, f in b.faces {
		if f == key {
			return i
		}
	}
	b.faces << key
	return b.faces.len - 1
}

pub fn (b &FakeBackend) metrics(face text.FaceId) text.FaceMetrics {
	return text.FaceMetrics{
		ascent:              0.8
		descent:             0.2
		underline_position:  0.1
		underline_thickness: 0.05
	}
}

pub fn (b &FakeBackend) shape(s string, face text.FaceId, size f32) []text.ShapedGlyph {
	mut out := []text.ShapedGlyph{}
	mut x := f32(0)
	adv := size * 0.6
	mut i := 0
	for i < s.len {
		j := text.next_grapheme(s, i)
		r := s[i..j].runes()[0]
		out << text.ShapedGlyph{
			face:    face
			id:      u16(u32(r) & 0xFFFF)
			x:       x
			advance: adv
			cluster: i
		}
		x += adv
		i = j
	}
	return out
}

pub fn (b &FakeBackend) raster(face text.FaceId, glyph u16, size_px f32, subpixel_x f32) !text.GlyphBitmap {
	if glyph == u16(` `) {
		return text.GlyphBitmap{}
	}
	w := int(size_px * 0.5 + 0.5)
	h := int(size_px * 0.7 + 0.5)
	return text.GlyphBitmap{
		left:   0
		top:    -h
		width:  w
		height: h
		pixels: []u8{len: w * h, init: 255}
	}
}

pub fn (mut b FakeBackend) contextual_off(_ string) {}

pub fn (mut b FakeBackend) font_features(_ string, _ []text.FontFeature) {}

pub fn (mut b FakeBackend) font_families() []string {
	return b.families.clone()
}

pub fn (mut b FakeBackend) load_fonts(files [][]u8) ![]string {
	mut names := []string{}
	for _ in files {
		names << 'Fake Bundled ${b.families.len}'
		b.families << names.last()
	}
	return names
}

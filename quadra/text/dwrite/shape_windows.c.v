// Shaping: UTF-16 run + font -> positioned glyphs (GetGlyphs + GetGlyphPlacements).
module dwrite

import quadra.platform.win

type FnGetGlyphs = fn (this voidptr, text &u16, len u32, face voidptr, sideways int, rtl int, sa &ScriptAnalysis, locale &u16, numsub voidptr, features voidptr, feature_lens voidptr, feature_ranges u32, max_glyphs u32, clusters &u16, text_props &u16, glyphs &u16, glyph_props &u16, actual &u32) int

type FnGetPlacements = fn (this voidptr, text &u16, clusters &u16, text_props &u16, len u32, glyphs &u16, glyph_props &u16, count u32, face voidptr, em f32, sideways int, rtl int, sa &ScriptAnalysis, locale &u16, features voidptr, feature_lens voidptr, feature_ranges u32, advances &f32, offsets &GlyphOffset) int

// FontFeature and TypoFeatures are DWRITE_FONT_FEATURE and DWRITE_TYPOGRAPHIC_FEATURES.
struct FontFeature {
	tag   u32
	param u32
}

struct TypoFeatures {
	features &FontFeature
	count    u32
}

const tag_calt = u32(`c`) | u32(`a`) << 8 | u32(`l`) << 16 | u32(`t`) << 24

// ShapedRun views the scratch buffers: it is valid until the next shape call.
pub struct ShapedRun {
pub:
	glyphs      []u16
	advances    []f32 // px at the shaping size
	offsets     []GlyphOffset
	clusters    []u16 // per UTF-16 unit: index of its first glyph
	glyph_props []u16 // DWRITE_SHAPING_GLYPH_PROPERTIES bitfield (bit 5 = diacritic)
}

pub fn (r &ShapedRun) width() f32 {
	mut w := f32(0)
	for a in r.advances {
		w += a
	}
	return w
}

// Scratch holds the buffers shaping reuses from call to call (shaping is single-threaded).
pub struct Scratch {
mut:
	clusters    []u16
	text_props  []u16
	glyphs      []u16
	glyph_props []u16
	advances    []f32
	offsets     []GlyphOffset
}

fn grow[T](mut a []T, n int) {
	if a.len < n {
		a = []T{len: n * 2}
	}
}

// shape shapes text[start..start+len] with one script analysis and one font.
pub fn (f &Factory) shape(text []u16, start u32, len u32, sa ScriptAnalysis, face &FontFace, size f32, rtl bool, loc &u16, mut sc Scratch) !ShapedRun {
	if len == 0 {
		return ShapedRun{}
	}
	p := unsafe { &u16(text.data) + start }
	grow(mut sc.clusters, int(len))
	grow(mut sc.text_props, int(len))
	mut max := len * 3 / 2 + 16
	mut count := u32(0)
	get_glyphs := unsafe { FnGetGlyphs(win.slot(f.analyzer, analyzer_get_glyphs)) }
	// One feature range over the whole run turns calt off for faces that ask for it.
	off := FontFeature{tag_calt, 0}
	set := TypoFeatures{&off, 1}
	sets := &set
	range_len := len
	mut feats, mut lens, mut ranges := unsafe { nil }, unsafe { nil }, u32(0)
	if face.no_calt {
		feats, lens, ranges = voidptr(&sets), voidptr(&range_len), 1
	}
	for {
		grow(mut sc.glyphs, int(max))
		grow(mut sc.glyph_props, int(max))
		max = u32(sc.glyphs.len)
		hr := get_glyphs(f.analyzer, p, len, face.face, 0, int(rtl), &sa, loc, unsafe { nil },
			feats, lens, ranges, max, sc.clusters.data, sc.text_props.data, sc.glyphs.data,
			sc.glyph_props.data, &count)
		if hr == e_not_sufficient_buffer {
			max *= 2
			continue
		}
		win.check(hr, 'GetGlyphs')!
		break
	}
	grow(mut sc.advances, int(count))
	grow(mut sc.offsets, int(count))
	place := unsafe { FnGetPlacements(win.slot(f.analyzer, analyzer_get_glyph_placements)) }
	win.check(place(f.analyzer, p, sc.clusters.data, sc.text_props.data, len, sc.glyphs.data,
		sc.glyph_props.data, count, face.face, size, 0, int(rtl), &sa, loc, feats, lens, ranges,
		sc.advances.data, sc.offsets.data), 'GetGlyphPlacements')!
	n := int(count)
	return ShapedRun{
		glyphs:      sc.glyphs[..n]
		advances:    sc.advances[..n]
		offsets:     sc.offsets[..n]
		clusters:    sc.clusters[..int(len)]
		glyph_props: sc.glyph_props[..n]
	}
}

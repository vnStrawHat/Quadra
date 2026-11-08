// TextSystem: font registry and the two-generation caches that make text cheap.
// An entry used during a frame survives into the next; anything unused for a whole
// frame is dropped by end_frame().
module text

import math
import quadra.gpu

struct Word {
mut:
	glyphs []ShapedGlyph
	width  f32
}

pub struct TextStats {
pub mut:
	words_shaped    int
	word_hits       int
	lines_built     int
	line_hits       int
	glyphs_rastered int
}

@[heap]
pub struct TextSystem {
mut:
	backend Backend
	faces   map[string]FaceId
	// one-entry memo of face_for
	memo_ok     bool
	memo_family string
	memo_weight int
	memo_italic bool
	memo_face   FaceId
	// Keyed by the text plus face/size bits (cache.v); entries live while used.
	words Cache[&Word]
	lines Cache[&LineLayout]
	wraps Cache[&WrappedText]
	tiles map[u64]GlyphTile
	// numbers (numeric.v): whether a face shapes them glyph by glyph, and its glyphs per size
	numeric_faces map[int]bool
	numeric_runs  map[u64][]ShapedGlyph
	// scratch reused on every call: a composed number (valid until the next word call) and
	// build_line's first x of each byte
	num      Word
	first_x  []f32
	families map[string]bool // has_family answers
pub mut:
	mono  gpu.Atlas = gpu.new_atlas(.mono, 2048)
	poly  gpu.Atlas = gpu.new_atlas(.poly, 2048)
	stats TextStats
}

pub fn new_system(backend Backend) &TextSystem {
	return &TextSystem{
		backend: backend
	}
}

pub fn (mut ts TextSystem) default_family() string {
	return ts.backend.default_family()
}

// load_font registers one bundled font and returns its family name; load_fonts takes a
// family's files together, which is cheaper than one by one.
pub fn (mut ts TextSystem) load_font(bytes []u8) !string {
	return ts.backend.load_fonts([bytes])![0]
}

pub fn (mut ts TextSystem) load_fonts(files [][]u8) ![]string {
	return ts.backend.load_fonts(files)
}

// has_family reports whether a family is installed or loaded (asked once per family).
pub fn (mut ts TextSystem) has_family(family string) bool {
	if known := ts.families[family] {
		return known
	}
	ts.backend.face(FontKey{family, 400, false}) or {
		ts.families[family] = false
		return false
	}
	ts.families[family] = true
	return true
}

pub fn (mut ts TextSystem) contextual_off(family string) {
	ts.backend.contextual_off(family)
}

// end_frame advances the cache generations (unused entries are swept, cache.v).
// cache_counts summarizes cache sizes (leak hunting).
pub fn (ts &TextSystem) cache_counts() string {
	return 'lines ${ts.lines.m.len} words ${ts.words.m.len} wraps ${ts.wraps.m.len} tiles ${ts.tiles.len}'
}

pub fn (mut ts TextSystem) end_frame() {
	ts.words.end_frame()
	ts.lines.end_frame()
	ts.wraps.end_frame()
}

pub fn (mut ts TextSystem) face_for(st TextStyle) FaceId {
	// Almost every call asks for the same face as the previous one.
	if ts.memo_ok && st.family == ts.memo_family && st.weight == ts.memo_weight
		&& st.italic == ts.memo_italic {
		return ts.memo_face
	}
	f := ts.lookup_face(st)
	ts.memo_ok, ts.memo_family, ts.memo_weight, ts.memo_italic, ts.memo_face = true, st.family, st.weight, st.italic, f
	return f
}

fn (mut ts TextSystem) lookup_face(st TextStyle) FaceId {
	family := if st.family == '' { ts.backend.default_family() } else { st.family }
	k := '${family}|${st.weight}|${st.italic}'
	if f := ts.faces[k] {
		return f
	}
	f := ts.backend.face(FontKey{family, st.weight, st.italic}) or {
		// Unknown family: fall back to the default family with the same weight.
		ts.backend.face(FontKey{ts.backend.default_family(), st.weight, st.italic}) or { 0 }
	}
	ts.faces[k] = f
	return f
}

// line_metrics returns ascent and descent in px for a style.
pub fn (mut ts TextSystem) line_metrics(st TextStyle) (f32, f32) {
	m := ts.backend.metrics(ts.face_for(st))
	return m.ascent * st.size, m.descent * st.size
}

pub fn (mut ts TextSystem) underline(st TextStyle) (f32, f32) {
	m := ts.backend.metrics(ts.face_for(st))
	return m.underline_position * st.size, m.underline_thickness * st.size
}

// pair packs two 32-bit values (a face id, a float's bits) into one cache key.
@[inline]
fn pair(a u32, b f32) u64 {
	return u64(a) << 32 | u64(math.f32_bits(b))
}

// word returns the shaped token. The token may view a larger string (substr_unsafe): the
// cache keeps a copy.
fn (mut ts TextSystem) word(token string, face FaceId, size f32) &Word {
	// Numbers are composed, which costs less than caching them (numeric.v).
	if token.len > 1 && is_numeric(token) && ts.numeric_ok(face) {
		return ts.compose_numeric(token, face, size)
	}
	key := pair(u32(face), size)
	if w := ts.words.get(token, key, 0) {
		ts.stats.word_hits++
		return w
	}
	w := word_of(ts.backend.shape(token, face, size))
	ts.words.put(token.clone(), key, 0, w)
	ts.stats.words_shaped++
	return w
}

fn word_of(glyphs []ShapedGlyph) &Word {
	mut width := f32(0)
	for g in glyphs {
		width += g.advance
	}
	return &Word{glyphs, width}
}

// token_end returns the end of the token at s[i] (before end): a single space or tab, or a
// maximal run of other characters (a stray newline is a token of its own).
@[inline]
fn token_end(s string, i int, end int) int {
	c := s[i]
	if c == ` ` || c == `\t` || c == `\n` {
		return i + 1
	}
	mut j := i + 1
	for j < end && s[j] != ` ` && s[j] != `\t` && s[j] != `\n` {
		j++
	}
	return j
}

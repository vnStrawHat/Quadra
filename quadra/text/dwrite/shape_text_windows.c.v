// text.Backend.shape: UTF-8 -> UTF-16, script runs, fallback fonts, then glyphs.
module dwrite

import quadra.text

// shape converts to UTF-16, itemizes by script and shapes each run. Glyph clusters are
// mapped back to UTF-8 byte offsets. Printable ASCII skips script analysis and font
// fallback: its analysis is known (one run, Latin when it has a letter) and every font has it.
pub fn (mut b DWriteBackend) shape(s string, face text.FaceId, size f32) []text.ShapedGlyph {
	if s.len == 0 {
		return []
	}
	if b.loc == unsafe { nil } || b.loc_of != b.locale {
		b.loc, b.loc_of = b.locale.to_wide(), b.locale
	}
	if letters := printable_ascii(s) {
		mut units := []u16{len: s.len, init: u16(s[index])}
		sa := b.ascii_analysis(letters)
		shaped := b.f.shape(units, 0, u32(s.len), sa, &b.faces[face], size, false, b.loc, mut
			b.scratch) or { return [] }
		mut out := []text.ShapedGlyph{cap: s.len}
		place_run(shaped, face, 0, 0, [], mut out)
		return out
	}
	return b.shape_analyzed(s, face, size)
}

// shape_analyzed is the general path: script analysis and font fallback for any text.
pub fn (mut b DWriteBackend) shape_analyzed(s string, face text.FaceId, size f32) []text.ShapedGlyph {
	if b.loc == unsafe { nil } || b.loc_of != b.locale {
		b.loc, b.loc_of = b.locale.to_wide(), b.locale
	}
	mut units := []u16{cap: s.len}
	mut unit_to_byte := []int{cap: s.len}
	mut i := 0
	for i < s.len {
		n := utf8_char_len(s[i])
		r := decode(s, i, n)
		if r >= 0x10000 {
			v := u32(r) - 0x10000
			units << u16(0xD800 + (v >> 10))
			units << u16(0xDC00 + (v & 0x3FF))
			unit_to_byte << i
			unit_to_byte << i
		} else {
			units << u16(r)
			unit_to_byte << i
		}
		i += n
	}
	analysis := b.f.analyze(units, b.loc) or { return [] }
	mut out := []text.ShapedGlyph{cap: units.len}
	mut pen := f32(0)
	key := b.keys[face]
	style := if key.italic { font_style_italic } else { font_style_normal }
	for run in analysis.scripts {
		// Characters the font lacks go to a fallback font (CJK, emoji, symbols); plain ASCII
		// skips the lookup.
		fonts := if ascii_only(units, run.start, run.len) {
			[FontRun{run.start, run.len, ''}]
		} else {
			b.f.map_fonts(units, run.start, run.len, key.family, key.weight, style, b.locale)
		}
		for fr in fonts {
			fid := if fr.family == '' {
				face
			} else {
				b.face(text.FontKey{fr.family, key.weight, key.italic}) or { face }
			}
			shaped := b.f.shape(units, fr.start, fr.len, run.analysis, &b.faces[fid], size, false,
				b.loc, mut b.scratch) or { continue }
			pen = place_run(shaped, fid, int(fr.start), pen, unit_to_byte, mut out)
		}
	}
	return out
}

// place_run appends a shaped run's glyphs at pen; unit_to_byte maps a UTF-16 unit to its UTF-8
// offset (empty: the same offset, for ASCII). It returns the pen after the run.
fn place_run(shaped ShapedRun, face text.FaceId, start int, pen0 f32, unit_to_byte []int, mut out []text.ShapedGlyph) f32 {
	// glyph -> first UTF-16 unit of its cluster
	mut glyph_unit := []int{len: shaped.glyphs.len, init: -1}
	for u := shaped.clusters.len - 1; u >= 0; u-- {
		g := int(shaped.clusters[u])
		if g < glyph_unit.len {
			glyph_unit[g] = u
		}
	}
	mut pen := pen0
	mut cur := 0
	for gi, gid in shaped.glyphs {
		if glyph_unit[gi] >= 0 {
			cur = glyph_unit[gi]
		}
		off := shaped.offsets[gi]
		out << text.ShapedGlyph{
			face:    face
			id:      gid
			x:       pen + off.advance_offset
			y:       off.ascender_offset
			advance: shaped.advances[gi]
			cluster: if unit_to_byte.len == 0 { start + cur } else { unit_to_byte[start + cur] }
		}
		pen += shaped.advances[gi]
	}
	return pen
}

// printable_ascii reports whether s is all printable ASCII, and if so whether it has a letter.
fn printable_ascii(s string) ?bool {
	mut letters := false
	for c in s {
		if c < 0x20 || c > 0x7e {
			return none
		}
		letters = letters || c.is_letter()
	}
	return letters
}

fn (mut b DWriteBackend) ascii_analysis(letters bool) ScriptAnalysis {
	if !b.ascii_ok {
		for k, probe in ['0', 'a'] {
			a := b.f.analyze([u16(probe[0])], b.loc) or { TextAnalysis{} }
			if a.scripts.len > 0 {
				b.ascii[k] = a.scripts[0].analysis
			}
		}
		b.ascii_ok = true
	}
	return b.ascii[int(letters)]
}

fn decode(s string, i int, n int) rune {
	c := u32(s[i])
	return match n {
		2 {
			rune((c & 0x1f) << 6 | u32(s[i + 1] & 0x3f))
		}
		3 {
			rune((c & 0x0f) << 12 | u32(s[i + 1] & 0x3f) << 6 | u32(s[i + 2] & 0x3f))
		}
		4 {
			rune((c & 0x07) << 18 | u32(s[i + 1] & 0x3f) << 12 | u32(s[i + 2] & 0x3f) << 6 | u32(s[
				i + 3] & 0x3f))
		}
		else {
			rune(c)
		}
	}
}

fn ascii_only(units []u16, start u32, len u32) bool {
	for i in start .. start + len {
		if units[i] >= 0x80 {
			return false
		}
	}
	return true
}

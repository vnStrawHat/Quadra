// Shaping fast paths on DirectWrite give the same glyphs as the general path.
module main

import quadra.text
import quadra.text.dwrite

const samples = ['Item 42', '(a)', '12.50%', '-3', 'AVATAR To.', 'fi fl ffi', 'x=1;y=2',
	'~!@#$^&*()_+{}|:"<>?', "Don't", 'WAVE', 'kernel 10.0.26200']

// Printable ASCII skips script analysis and font fallback (dwrite/shape_text_windows.c.v).
fn test_ascii_fast_path_matches_full_analysis() {
	mut b := dwrite.new_backend()!
	for fam in ['Segoe UI', 'Cascadia Mono', 'Georgia'] {
		face := b.face(text.FontKey{fam, 400, false})!
		for s in samples {
			fast := b.shape(s, face, 13)
			full := b.shape_analyzed(s, face, 13)
			assert fast == full, '${fam}: "${s}"'
		}
	}
}

// Numbers are composed from glyphs shaped once (text/numeric.v) when the face allows it.
fn test_composed_numbers_match_shaping() {
	mut b := dwrite.new_backend()!
	mut ts := text.new_system(b)
	st := text.TextStyle{
		size: 13
	}
	face := ts.face_for(st)
	for s in ['0.00', '123.45', '-12,5%', '10:42:07', '3/4', '+1.5'] {
		l := ts.layout_line(s, st)
		ref := b.shape(s, face, 13)
		assert l.glyphs.len == ref.len, s
		for i, g in l.glyphs {
			assert g.id == ref[i].id, s
			d := g.x - ref[i].x
			assert d < 0.001 && d > -0.001, '${s}: glyph ${i} at ${g.x}, shaped at ${ref[i].x}'
		}
	}
}

// Wrapped lines are cut from their paragraph's layout (text/line_layout.v cut): each must
// read exactly like the same text laid out on its own.
fn test_cut_lines_match_lines_laid_out_alone() {
	mut ts := text.new_system(dwrite.new_backend()!)
	st := text.TextStyle{
		size: 13
	}
	s := 'Rolling restarts wait for readiness probes; Tiếng Việt có dấu, AVATAR To. fi fl office 12.50% done.'
	for max_w in [f32(60), 90, 140, 230] {
		w := ts.wrap(s, st, max_w)
		assert w.lines.len > 1
		for line in w.lines {
			cut := line.layout
			alone := ts.layout_line(s[line.start..line.end], st)
			assert cut.text == alone.text
			assert f32_near(cut.width, alone.width), '${max_w}: "${cut.text}"'
			assert cut.glyphs.len == alone.glyphs.len
			for i, g in cut.glyphs { // painting reads these, shifted by origin and base
				assert f32_near(g.x - cut.origin, alone.glyphs[i].x)
				assert g.cluster - cut.base == alone.glyphs[i].cluster
				assert g.id == alone.glyphs[i].id
			}
			for i in 0 .. cut.text.len + 1 {
				assert f32_near(cut.x_for_index(i), alone.x_for_index(i))
			}
			for x := f32(-2); x < alone.width + 4; x += 3 {
				assert cut.index_for_x(x) == alone.index_for_x(x)
			}
		}
	}
}

fn f32_near(a f32, b f32) bool {
	return a - b < 0.01 && b - a < 0.01
}

// A word wraps alone at its own measured width, even after float error in layout sums.
fn test_word_fits_its_measured_width() {
	mut b := dwrite.new_backend()!
	mut ts := text.new_system(b)
	st := text.TextStyle{
		size:   12
		weight: 500
	}
	for s in ['Preview', 'Split', 'Raw', 'Markdown'] {
		w := ts.wrap(s, st, 1.0e9).width
		assert ts.wrap(s, st, w - 0.0001).lines.len == 1, s
	}
}

// font_features changes the glyphs a family is shaped with, and back.
fn test_font_features_change_the_glyphs() {
	mut b := dwrite.new_backend()!
	face := b.face(text.FontKey{'Cascadia Mono', 400, false})!
	plain := b.shape('0', face, 13)
	b.font_features('Cascadia Mono', [text.FontFeature{'zero', 1}])
	feat := b.shape('0', face, 13)
	assert plain[0].id != feat[0].id
	b.font_features('Cascadia Mono', []text.FontFeature{})
	assert b.shape('0', face, 13)[0].id == plain[0].id
}

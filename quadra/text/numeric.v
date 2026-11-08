// Numbers are the text that changes every frame (tables, meters, logs), so each new one
// would be shaped. When a face shapes them glyph by glyph (no kerning or ligatures between
// any two of their characters, checked once per face), a number is composed from glyphs
// shaped once per size instead.
module text

const numeric_chars = '0123456789.,-+%:/'

fn numeric_index(c u8) int {
	return numeric_chars.index_u8(c)
}

fn is_numeric(s string) bool {
	for c in s {
		if numeric_index(c) < 0 {
			return false
		}
	}
	return true
}

// numeric_ok checks every pair of numeric characters once per face: each pair must shape to
// the two glyphs of its characters, the second one advanced by the first's width.
// It runs once per face; the shaped singles stay in scope across the pair calls (specs/14,
// GC pins), which is fine at 289 calls.
fn (mut ts TextSystem) numeric_ok(face FaceId) bool {
	if ok := ts.numeric_faces[face] {
		return ok
	}
	size := f32(16)
	singles := numeric_chars.split('').map(ts.backend.shape(it, face, size))
	mut ok := singles.all(it.len == 1)
	for i in 0 .. numeric_chars.len {
		for j in 0 .. numeric_chars.len {
			if !ok {
				break
			}
			a, b := singles[i][0], singles[j][0]
			p := ts.backend.shape(numeric_chars[i..i + 1] + numeric_chars[j..j + 1], face, size)
			ok = p.len == 2 && p[0].id == a.id && p[1].id == b.id && p[0].advance == a.advance
				&& p[1].advance == b.advance && f32_near(p[1].x, a.advance + b.x)
		}
	}
	ts.numeric_faces[face] = ok
	return ok
}

fn f32_near(a f32, b f32) bool {
	d := a - b
	return d < 0.001 && d > -0.001
}

// compose_numeric lays out a number into ts.num, which the next call reuses.
fn (mut ts TextSystem) compose_numeric(s string, face FaceId, size f32) &Word {
	key := pair(u32(face), size)
	if key !in ts.numeric_runs {
		ts.numeric_runs[key] = numeric_chars.split('').map(ts.backend.shape(it, face, size)[0])
	}
	singles := ts.numeric_runs[key] or { return &ts.num }
	ts.num.glyphs.clear()
	mut x := f32(0)
	for i, c in s {
		g := singles[numeric_index(c)]
		ts.num.glyphs << ShapedGlyph{g.face, g.id, x + g.x, g.y, g.advance, i}
		x += g.advance
	}
	ts.num.width = x
	return &ts.num
}

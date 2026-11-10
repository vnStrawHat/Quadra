// Fuzz the SVG reader, path data and the rasterizer (specs/11, fuzz tests): icons and images
// can come from untrusted files, so any markup must parse and render without a panic or a hang.
module vector

import math
import strings
import quadra.prim
import quadra.testing.fuzz

const svg_corpus = [
	'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M12 5l7 7-7 7"/></svg>',
	'<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="6" fill="#f00"/><rect x="1" y="1" width="4" height="4" rx="1"/></svg>',
	'<?xml version="1.0"?><!-- c --><svg viewBox="-2 -2 20 20"><g fill="#0af" stroke="white"><ellipse cx="5" cy="5" rx="4" ry="2"/><line x1="0" y1="0" x2="9" y2="9"/></g></svg>',
	'<svg><polygon points="1,1 9,1 5,8"/><polyline points="0 0 4 4 8 0" stroke="#abc" fill="none"/></svg>',
	'<svg viewBox="0 0 24 24"><path d="M2 2C5 0 9 0 12 2S19 4 22 2Q20 8 22 12T22 20a4 4 0 1 1-8 0A6 3 30 011 4 4zH2V10z" fill="currentColor"/></svg>',
	'<svg viewBox="0 0 10 10"><path d="m1 1 2 2 3-3l.5.5.5-.5e1 1e-1 1E+1h-1v-1c1 1 1 1 1 1s1 1 1 1q1 1 1 1t1 1z"/></svg>',
]

fn deep(mut r fuzz.Rng) string {
	n := 50 + r.intn(3000)
	match r.intn(5) {
		0 {
			return '<svg>' + '<g fill="#123">'.repeat(n) + '<path d="M0 0L5 5Z"/>' +
				'</g>'.repeat(r.intn(n))
		}
		1 {
			return '<svg><path d="M0 0' + ' L1e30 -1e30 a1e30 1 0 1 1 1e-30 1e38'.repeat(n / 10) +
				'Z"/></svg>'
		}
		2 {
			return '<svg><polygon points="' + strings.repeat(`9`, n) + ' 0 0 ' + '1 '.repeat(n) +
				'"/></svg>'
		}
		3 {
			return '<svg viewBox="' + r.pick(fuzz.boundary_texts) + ' 0 ' +
				r.pick(fuzz.boundary_texts) + ' ' + r.pick(fuzz.boundary_texts) +
				'"><path d="M0 0h9v9z" stroke-width="' + r.pick(fuzz.boundary_texts) +
				'" stroke="#fff"/></svg>'
		}
		else {
			return '<'.repeat(n) + '=' + '"'.repeat(n) + '>'
		}
	}
}

fn check_svg(src string) {
	s := parse_svg(src) or { return }
	_ = s.render_mask(16, 16)
	_ = s.render_rgba(12, 9, prim.white)
	_ = s.is_single_color()
	for sh in s.shapes {
		_ = sh.path.hash()
		_ = sh.path.flatten(3.5, -1, 2, 0.1)
	}
}

fn test_fuzz_svg() {
	corpus := svg_corpus.map(it.bytes())
	for c in svg_corpus {
		s := parse_svg(c)!
		assert s.shapes.len > 0, c
		check_svg(c)
	}
	// Regressions: an attribute with no name before `=` (it took the last of no fields), and
	// a zero viewBox, non-finite or huge coordinates: NaN, and int() of a float past 2^31
	// (INT_MIN), made the rasterizer loop over about 2^31 rows or columns.
	check_svg('<svg ="a"><path d="M0 0L9 9Z" x="1"="2"/></svg>')
	for vb in ['0 0 0 0', '0 0 -5 10', '0 0 1e39 1e39', '1e39 0 1 1'] {
		s := parse_svg('<svg viewBox="${vb}"><path d="M0 0L10 10L0 10Z" stroke="#fff"/></svg>')!
		assert s.render_mask(16, 16).len == 256
	}
	mut c := new_coverage(8, 8)
	nan := f32(math.nan())
	c.fill([
		Polyline{
			points: [prim.pt(0, nan), prim.pt(4, 9), prim.pt(nan, 1e30)]
		},
	])
	c.stroke([Polyline{
		points: [prim.pt(nan, 0), prim.pt(4, nan)]
	}], 1e30)
	c.fill([
		Polyline{
			points: [prim.pt(0, -1e30), prim.pt(5, 1e30), prim.pt(3, 3)]
		},
	])
	c.stroke([Polyline{
		points: [prim.pt(1e30, 1), prim.pt(2, -1e30)]
	}], 2)
	n := fuzz.iters(2000)
	mut r := fuzz.new_rng(0x737667)
	mut w := fuzz.watch('svg', n)
	for i in 0 .. n {
		mut src := ''
		if r.chance(10) {
			src = deep(mut r)
		} else {
			src = r.mutate(corpus[r.intn(corpus.len)], corpus).bytestr()
		}
		w.start()
		check_svg(src)
		w.done(i, src.len, 2000)
	}
	w.finish(64)
}

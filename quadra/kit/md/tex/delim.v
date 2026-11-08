// Delimiters that grow with their content, and matrices. Small sizes use the font's glyph
// (scaled up a little); taller ones are drawn as strokes.
module tex

import math
import quadra.prim

fn (mut l Lay) delimited(n Node, st int) Box {
	return l.wrap(l.list(n.body, st), n.text, n.text2, st)
}

// wrap puts grown delimiters left and right of inner (TeX: they cover 90 % of the height
// around the axis, and at least all of it but a quarter em).
fn (mut l Lay) wrap(inner Box, left string, right string, st int) Box {
	ax := l.axis(st)
	ext := prim.max(inner.h - ax, inner.d + ax)
	half := prim.max(ext * 0.901, ext - l.em(st) * 0.25)
	lb := l.delim_box(left, half, st)
	rb := l.delim_box(right, half, st)
	mut b := Box{
		w: lb.w + inner.w + rb.w
	}
	b.put(lb, 0, 0)
	b.grow(lb, 0)
	b.put(inner, lb.w, 0)
	b.grow(inner, 0)
	b.put(rb, lb.w + inner.w, 0)
	b.grow(rb, 0)
	return b
}

// delim_box is delimiter d covering half above and below the axis.
fn (mut l Lay) delim_box(d string, half f32, st int) Box {
	em := l.em(st)
	if d == '' {
		return Box{
			w: em * 0.12
		}
	}
	ax := l.axis(st)
	g := l.glyph(d, st)
	ink := g.h + g.d
	scale := if ink > 0 { 2 * half / ink } else { f32(1) }
	if scale <= 1.6 {
		mut gg := g
		if scale > 1.05 {
			gg = l.glyph_at(d, em * scale)
		}
		shift := (gg.h - gg.d) / 2 - ax
		mut b := Box{
			w: gg.w
			h: gg.h - shift
			d: gg.d + shift
		}
		b.put(gg, 0, shift)
		return b
	}
	return l.drawn(d, half, st)
}

// quad samples a quadratic curve from a over c to b.
fn quad(mut pts []prim.Point, a prim.Point, c prim.Point, b prim.Point) {
	for i in 0 .. 13 {
		t := f32(i) / 12
		s := 1 - t
		pts << prim.pt(s * s * a.x + 2 * s * t * c.x + t * t * b.x, s * s * a.y + 2 * s * t * c.y +
			t * t * b.y)
	}
}

// drawn draws a tall delimiter as strokes: center on the axis, half above and below.
fn (mut l Lay) drawn(d string, half f32, st int) Box {
	em := l.em(st)
	ax := l.axis(st)
	lw := em * 0.08
	w := prim.min(em * 0.32 + half * 0.08, em * 0.6)
	side := em * 0.08
	top, bot := -ax - half, -ax + half
	mid := -ax
	x0, x1 := side, side + w
	mut b := Box{
		w: w + 2 * side
		h: ax + half
		d: half - ax
	}
	mirror := d in [')', ']', '}', '⟩', '⌋', '⌉']
	mut pts := []prim.Point{}
	match d {
		'(', ')' {
			quad(mut pts, prim.pt(x1, top), prim.pt(x0 - w * 0.9, mid), prim.pt(x1, bot))
		}
		'[', ']' {
			pts = [prim.pt(x1, top), prim.pt(x0 + lw, top), prim.pt(x0 + lw, bot),
				prim.pt(x1, bot)]
		}
		'⌊', '⌋' {
			pts = [prim.pt(x0 + lw, top), prim.pt(x0 + lw, bot),
				prim.pt(x1, bot)]
		}
		'⌈', '⌉' {
			pts = [prim.pt(x1, top), prim.pt(x0 + lw, top), prim.pt(x0 + lw, bot)]
		}
		'⟨', '⟩' {
			pts = [prim.pt(x1, top), prim.pt(x0, mid), prim.pt(x1, bot)]
		}
		'{', '}' {
			xm := (x0 + x1) / 2
			quad(mut pts, prim.pt(x1, top), prim.pt(xm, top), prim.pt(xm, top + half * 0.3))
			quad(mut pts, prim.pt(xm, mid - half * 0.25), prim.pt(xm, mid), prim.pt(x0, mid))
			mut lower := []prim.Point{}
			quad(mut lower, prim.pt(x0, mid), prim.pt(xm, mid), prim.pt(xm, mid + half * 0.25))
			quad(mut lower, prim.pt(xm, bot - half * 0.3), prim.pt(xm, bot), prim.pt(x1, bot))
			pts << lower
		}
		'‖' {
			b.stroke([prim.pt(x0 + w * 0.3, top), prim.pt(x0 + w * 0.3, bot)], lw)
			pts = [prim.pt(x0 + w * 0.7, top), prim.pt(x0 + w * 0.7, bot)]
		}
		'/' {
			pts = [prim.pt(x1, top), prim.pt(x0, bot)]
		}
		else { // | and anything else: a bar
			pts = [prim.pt((x0 + x1) / 2, top), prim.pt((x0 + x1) / 2, bot)]
		}
	}

	if mirror {
		pts = pts.map(prim.pt(x0 + x1 - it.x, it.y))
	}
	b.stroke(pts, lw)
	return b
}

const aligned_envs = ['aligned', 'align', 'align*', 'split']

// matrix lays out rows of cells centered on the axis, with the environment's delimiters.
fn (mut l Lay) matrix(n Node, st int) Box {
	aligned := n.env in aligned_envs
	cst := if aligned || n.env == 'gathered' {
		st
	} else if n.env == 'smallmatrix' {
		2
	} else {
		1
	}
	em := l.em(st)
	mut ncols := 0
	mut cells := [][]Box{}
	for row in n.rows {
		mut out := []Box{}
		for j, cell in row {
			// align: the right column starts as if after an ordinary atom ("&= b" spaces the =).
			mut body := []Node{}
			if aligned && j % 2 == 1 {
				body << Node{
					kind: .group
				}
			}
			body << cell
			out << l.list(body, cst)
		}
		ncols = int(math.max(ncols, out.len))
		cells << out
	}
	mut widths := []f32{len: ncols}
	for row in cells {
		for j, c in row {
			widths[j] = prim.max(widths[j], c.w)
		}
	}
	gap := if aligned {
		f32(0)
	} else if n.env == 'cases' {
		em
	} else {
		em * 0.9
	}
	row_gap := if aligned { em * 0.3 } else { em * 0.12 }
	mut b := Box{}
	mut y := f32(0) // top of the current row
	for row in cells {
		mut h, mut d := em * 0.75, em * 0.3
		for c in row {
			h, d = prim.max(h, c.h), prim.max(d, c.d)
		}
		mut x := f32(0)
		for j, c in row {
			cx := match true {
				aligned && j % 2 == 0 { x + widths[j] - c.w }
				aligned || n.env == 'cases' { x }
				else { x + (widths[j] - c.w) / 2 }
			}

			b.put(c, cx, y + h)
			x += widths[j] + if aligned && j % 2 == 1 { em * 2 } else { gap }
		}
		y += h + d + row_gap
	}
	total := y - row_gap
	for j, w in widths {
		b.w += w + if j + 1 < ncols {
			if aligned && j % 2 == 1 {
				em * 2
			} else if aligned {
				0
			} else {
				gap
			}
		} else {
			0
		}
	}
	// Center the rows on the axis.
	shift := -(total / 2 + l.axis(st))
	mut c := Box{
		w: b.w
		h: -shift
		d: total + shift
	}
	c.put(b, 0, shift)
	left, right := match n.env {
		'pmatrix' { '(', ')' }
		'bmatrix' { '[', ']' }
		'Bmatrix' { '{', '}' }
		'vmatrix' { '|', '|' }
		'Vmatrix' { '‖', '‖' }
		'cases' { '{', '' }
		else { '', '' }
	}

	if left == '' && right == '' {
		return c
	}
	pad := Box{
		w: em * 0.15
	}
	mut padded := Box{
		w: c.w + pad.w * 2
		h: c.h
		d: c.d
	}
	padded.put(c, pad.w, 0)
	return l.wrap(padded, left, right, st)
}

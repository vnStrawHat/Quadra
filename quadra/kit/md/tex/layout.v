// Box layout in the TeX way: every node becomes a box (width, height above the baseline,
// depth below) holding positioned glyphs, rules and strokes. Coordinates have the
// baseline at y = 0 and y growing down.
module tex

import quadra.prim
import quadra.text

pub const math_family = 'Cambria Math'

enum ItemKind {
	glyph
	rule
	stroke
}

struct Item {
	kind  ItemKind
	x     f32
	y     f32 // the baseline (glyph) or the top edge (rule)
	w     f32 // rule width, or stroke width
	h     f32
	line  &text.LineLayout = unsafe { nil }
	pts   []prim.Point
	error bool // painted in the danger color
}

pub struct Box {
pub mut:
	w f32
	h f32
	d f32
mut:
	ic    f32 // italic correction: ink past the advance (superscripts move right by it)
	lo    f32 = -1e9 // a glyph's ink bottom below the baseline, negative when above it
	items []Item
}

// put places c with its baseline at (x, y) inside b (items only; b's size is the caller's).
fn (mut b Box) put(c Box, x f32, y f32) {
	for item in c.items {
		b.items << Item{
			...item
			x:   item.x + x
			y:   item.y + y
			pts: item.pts.map(prim.pt(it.x + x, it.y + y))
		}
	}
}

// grow extends b's height and depth to hold c placed with its baseline at y.
fn (mut b Box) grow(c Box, y f32) {
	b.h = prim.max(b.h, c.h - y)
	b.d = prim.max(b.d, c.d + y)
}

fn (mut b Box) rule(x f32, top f32, w f32, h f32) {
	b.items << Item{
		kind: .rule
		x:    x
		y:    top
		w:    w
		h:    h
	}
}

fn (mut b Box) stroke(pts []prim.Point, w f32) {
	b.items << Item{
		kind: .stroke
		w:    w
		pts:  pts
	}
}

// Lay lays out nodes at a base font size.
struct Lay {
mut:
	ts     &text.TextSystem = unsafe { nil }
	size   f32
	family string = math_family
	axes   map[int]f32
}

const style_scale = [f32(1), 1, 0.7, 0.5]

fn (l &Lay) em(st int) f32 {
	return l.size * style_scale[st]
}

fn (l &Lay) mu(st int) f32 {
	return l.em(st) / 18
}

// rule_w is the thickness of fraction bars and radical tops.
fn (l &Lay) rule_w(st int) f32 {
	return prim.max(0.6, l.em(st) * 0.055)
}

fn script_style(st int) int {
	return if st < 2 { 2 } else { 3 }
}

// glyph shapes s and measures its ink, which TeX layout needs (font ascents are far too
// tall for Cambria Math).
fn (mut l Lay) glyph(s string, st int) Box {
	return l.glyph_at(s, l.em(st))
}

fn (mut l Lay) glyph_at(s string, size f32) Box {
	if s == '' {
		return Box{}
	}
	line := l.ts.layout_line(s, text.TextStyle{
		family: l.family
		size:   size
	})
	mut b := Box{
		w: line.width
	}
	mut right := f32(0)
	for g in line.glyphs {
		tile := l.ts.glyph_tile(g.face, g.id, size, 0)
		if tile.empty {
			continue
		}
		b.h = prim.max(b.h, g.y - f32(tile.top))
		b.d = prim.max(b.d, f32(tile.top + tile.tile.h) - g.y)
		b.lo = prim.max(b.lo, f32(tile.top + tile.tile.h) - g.y)
		right = prim.max(right, g.x + f32(tile.left + tile.tile.w))
	}
	b.ic = prim.max(0, right - line.width - 0.5)
	b.items << Item{
		kind: .glyph
		line: line
	}
	return b
}

// axis is the height of the math axis (fraction bars, operator centers) above the baseline.
fn (mut l Lay) axis(st int) f32 {
	key := int(l.em(st) * 64)
	if a := l.axes[key] {
		return a
	}
	m := l.glyph('−', st)
	a := if m.h > 0 { (m.h - m.d) / 2 } else { l.em(st) * 0.25 }
	l.axes[key] = a
	return a
}

// Inter-atom spaces (TeX's table, in thin / medium / thick units); negative ones apply in
// display and text style only.
const spacing = [
	[0, 1, -2, -3, 0, 0, 0, -1],
	[1, 1, 0, -3, 0, 0, 0, -1],
	[-2, -2, 0, 0, -2, 0, 0, -2],
	[-3, -3, 0, 0, -3, 0, 0, -3],
	[0, 0, 0, 0, 0, 0, 0, 0],
	[0, 1, -2, -3, 0, 0, 0, -1],
	[-1, -1, 0, -1, -1, -1, -1, -1],
	[-1, 1, -2, -3, -1, 0, -1, -1],
]

fn class_of(n Node) Cls {
	return match n.kind {
		.scripts {
			if n.body.len > 0 {
				class_of(n.body[0])
			} else {
				Cls.ord
			}
		}
		.frac {
			Cls.inner
		}
		.group, .sqrt, .accent, .text {
			Cls.ord
		}
		else {
			n.cls
		}
	}
}

// list lays out nodes side by side with TeX's spacing.
fn (mut l Lay) list(nodes []Node, st0 int) Box {
	mut st := st0
	mut cls := []Cls{len: nodes.len}
	for i, n in nodes {
		cls[i] = class_of(n)
	}
	// A binary operator with nothing to operate on is ordinary (-x, a = -b).
	for i, c in cls {
		if c != .bin {
			continue
		}
		prev := if i > 0 { cls[i - 1] } else { Cls.open }
		next := if i + 1 < cls.len { cls[i + 1] } else { Cls.close }
		if prev in [.bin, .op, .rel, .open, .punct] || next in [.rel, .close, .punct] {
			cls[i] = .ord
		}
	}
	mut b := Box{}
	mut prev := -1
	for i, n in nodes {
		match n.kind {
			.style {
				st = n.level
				continue
			}
			.space {
				b.w += n.amount * l.mu(st)
				continue
			}
			else {}
		}

		c := l.node(n, st)
		k := int(cls[i])
		if prev >= 0 {
			gap := spacing[prev][k]
			if gap > 0 || (gap < 0 && st < 2) {
				units := if gap < 0 { -gap } else { gap }
				b.w += f32(units + 2) * l.mu(st)
			}
		}
		b.put(c, b.w, 0)
		b.grow(c, 0)
		b.w += c.w
		b.ic = c.ic
		prev = k
	}
	return b
}

fn (mut l Lay) node(n Node, st int) Box {
	return match n.kind {
		.atom, .text { l.glyph(n.text, st) }
		.group { l.list(n.body, st) }
		.frac { l.frac(n, st) }
		.sqrt { l.sqrt(n, st) }
		.scripts { l.scripts(n, st) }
		.bigop { l.bigop(n, st) }
		.fn_name { l.glyph(n.text, st) }
		.delim { l.delimited(n, st) }
		.big { l.delim_box(n.text, [f32(1.2), 1.8, 2.4, 3.0][n.level - 1] * l.em(st) / 2, st) }
		.accent { l.accent(n, st) }
		.matrix { l.matrix(n, st) }
		.stack { l.limits(l.list(n.body, st), n, st) }
		.space, .style { Box{} }
	}
}

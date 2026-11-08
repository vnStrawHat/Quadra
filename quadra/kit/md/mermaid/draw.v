// Drawing: what a laid-out diagram paints, as a flat list of operations in diagram space
// (logical px, origin top-left). Layout produces it once; painting walks it. Colors are
// roles resolved against the theme at paint time, so a theme switch needs no relayout.
module mermaid

import quadra.prim
import quadra.text

enum Ink {
	none
	panel  // node fill
	canvas // edge label background
	line   // node and frame borders, lifelines
	text
	text_2    // secondary text (frame titles)
	accent    // edges and arrows
	frame     // subgraph and frame fill
	on_accent // text on the accent (sequence numbers)
	note      // sequence notes
}

enum OpKind {
	box  // rect with fill, border and radius
	poly // polyline stroke (pts), optionally dashed or closed
	fill // filled polygon (pts) with an optional border
	text // one line: rect.x left, rect.y baseline
	head // arrowhead: pts[0] tip, pts[1] a point behind it on the line
}

pub enum Head {
	none
	arrow // filled triangle
	open  // two strokes (async message)
	circle
	cross
}

struct Op {
	kind   OpKind
	rect   prim.Rect
	radius f32
	bg     Ink
	border Ink
	pts    []prim.Point
	width  f32 = 1
	dash   bool
	closed bool
	text   string
	size   f32
	weight int = 400
	ink    Ink
	head   Head
}

struct Drawing {
mut:
	ops    []Op
	width  f32
	height f32
}

// Measure measures text in the diagram's font. base is the node text style.
struct Measure {
mut:
	ts   &text.TextSystem = unsafe { nil }
	base text.TextStyle
}

fn (mut m Measure) style(size f32, weight int) text.TextStyle {
	return text.TextStyle{
		...m.base
		size:   size
		weight: weight
	}
}

fn (mut m Measure) width(s string, size f32, weight int) f32 {
	if s == '' {
		return 0
	}
	return m.ts.layout_line(s, m.style(size, weight)).width
}

// line_h is the advance between lines of text at size.
fn (m &Measure) line_h(size f32) f32 {
	return f32(int(size * 1.4 + 0.5))
}

// TextBox is measured multi-line text (lines split at newlines).
struct TextBox {
	lines  []string
	widths []f32
	width  f32
	height f32
	size   f32
	weight int
}

fn (mut m Measure) box(s string, size f32, weight int) TextBox {
	lines := if s == '' { []string{} } else { s.split('\n') }
	mut widths := []f32{cap: lines.len}
	mut w := f32(0)
	for l in lines {
		lw := m.width(l, size, weight)
		widths << lw
		w = prim.max(w, lw)
	}
	return TextBox{lines, widths, w, m.line_h(size) * lines.len, size, weight}
}

// text_ops centers the lines of tb on (cx, cy).
fn (mut d Drawing) text_ops(m &Measure, tb TextBox, cx f32, cy f32, ink Ink) {
	lh := m.line_h(tb.size)
	top := cy - tb.height / 2
	for i, l in tb.lines {
		// The baseline sits at about 0.72 of the line advance for UI fonts at 1.4 leading.
		d.ops << Op{
			kind:   .text
			rect:   prim.rect(cx - tb.widths[i] / 2, top + lh * i + lh * 0.72, tb.widths[i], lh)
			text:   l
			size:   tb.size
			weight: tb.weight
			ink:    ink
		}
	}
}

// text_left puts one line with its left end at x and its vertical center at cy.
fn (mut d Drawing) text_left(m &Measure, s string, x f32, cy f32, size f32, weight int, ink Ink) {
	lh := m.line_h(size)
	d.ops << Op{
		kind:   .text
		rect:   prim.rect(x, cy - lh / 2 + lh * 0.72, 0, lh)
		text:   s
		size:   size
		weight: weight
		ink:    ink
	}
}

fn (mut d Drawing) poly(pts []prim.Point, width f32, dash bool, ink Ink) {
	d.ops << Op{
		kind:  .poly
		pts:   pts
		width: width
		dash:  dash
		ink:   ink
	}
}

fn (mut d Drawing) head(tip prim.Point, from prim.Point, h Head, ink Ink) {
	if h == .none {
		return
	}
	d.ops << Op{
		kind: .head
		pts:  [tip, from]
		head: h
		ink:  ink
	}
}

// decode turns label markup into plain text: `<br>` breaks lines, quotes are dropped.
fn decode(s string) string {
	mut t := s.trim_space()
	if t.len >= 2 && t[0] == `"` && t[t.len - 1] == `"` {
		t = t[1..t.len - 1]
	}
	return t.replace_each(['<br/>', '\n', '<br />', '\n', '<br>', '\n', '\\n', '\n', '&quot;',
		'"', '&amp;', '&', '&lt;', '<', '&gt;', '>'])
}

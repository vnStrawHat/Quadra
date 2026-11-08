// Formula: TeX math as an element (display math) and as an inline box in rich text
// (ui.InlineBox). The source is parsed once; the layout is kept per font size.
module tex

import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.layout
import quadra.text

@[heap]
pub struct Formula {
pub:
	src     string
	display bool
mut:
	nodes     []Node
	err       string
	px        f32
	tint      prim.Color
	tinted    bool
	laid      Box
	laid_size f32 = -1
	node      layout.NodeId
}

// formula parses src (TeX math, no surrounding $) once; display picks display style.
pub fn formula(src string, display bool) &Formula {
	nodes, err := parse(src)
	return &Formula{
		src:     src
		display: display
		nodes:   nodes
		err:     err
	}
}

// size sets the font size; 0 (the default) takes the inherited text size.
pub fn (self_ &Formula) size(px f32) &Formula {
	mut f := unsafe { self_ }
	f.px = px
	return f
}

// color sets the ink; by default the inherited text color.
pub fn (self_ &Formula) color(c prim.Color) &Formula {
	mut f := unsafe { self_ }
	f.tint, f.tinted = c, true
	return f
}

// error is the parse error, if any; the formula then shows its source in the danger color.
pub fn (f &Formula) error() ?string {
	return if f.err == '' { none } else { f.err }
}

// measure lays the formula out at size (kept until the size changes) and returns its box.
pub fn (mut f Formula) measure(ts &text.TextSystem, size f32) Box {
	px := if f.px > 0 { f.px } else { size }
	if f.laid_size == px {
		return f.laid
	}
	mut l := Lay{
		ts:   unsafe { ts }
		size: px
	}
	st := if f.display { 0 } else { 1 }
	if f.err != '' {
		f.laid = l.error_box(f.src)
	} else {
		f.laid = l.list(f.nodes, st)
	}
	f.laid_size = px
	return f.laid
}

fn (mut l Lay) error_box(src string) Box {
	l.family = '' // the interface font: the source is shown as typed
	g := l.glyph(src, 1)
	mut b := Box{
		w: g.w
		h: g.h
		d: g.d
	}
	for it in g.items {
		b.items << Item{
			...it
			error: true
		}
	}
	return b
}

pub fn (mut f Formula) request_layout(mut cx core.LayoutCx) layout.NodeId {
	b := f.measure(cx.text, cx.text_style.size)
	f.node = cx.tree.node(layout.Style{
		width:  layout.px(b.w)
		height: layout.px(b.h + b.d)
		shrink: 0
	}, [])
	return f.node
}

pub fn (mut f Formula) prepaint(_ prim.Rect, mut _ core.PrepaintCx) {}

pub fn (mut f Formula) paint(bounds prim.Rect, mut cx core.PaintCx) {
	c := if f.tinted { f.tint } else { cx.text_style.color }
	paint_box_items(f.laid, prim.pt(bounds.x, bounds.y + f.laid.h), c, mut cx)
}

// box_size is the inline size (ui.InlineBox): width, height above the baseline, depth.
// The layout it makes is a cache, hence the mutable view of an immutable receiver.
pub fn (self_ &Formula) box_size(mut ts text.TextSystem, st text.TextStyle) (f32, f32, f32) {
	mut f := unsafe { self_ }
	b := f.measure(&ts, st.size)
	return b.w, b.h, b.d
}

// paint_box paints the formula inline with its baseline's left end at origin.
pub fn (mut f Formula) paint_box(origin prim.Point, st text.TextStyle, mut cx core.PaintCx) {
	b := f.measure(cx.text, st.size)
	paint_box_items(b, origin, if f.tinted { f.tint } else { st.color }, mut cx)
}

fn paint_box_items(b Box, o prim.Point, color prim.Color, mut cx core.PaintCx) {
	danger := kit.theme_of(cx.app).colors.danger
	for item in b.items {
		c := if item.error { danger } else { color }
		match item.kind {
			.glyph {
				cx.glyphs(item.line, o.x + item.x, o.y + item.y, c)
			}
			.rule {
				cx.quad(gpu.Quad{
					bounds:     prim.rect(o.x + item.x, o.y + item.y, item.w, item.h)
					background: c
				})
			}
			.stroke {
				cx.stroke(item.pts.map(prim.pt(o.x + it.x, o.y + it.y)), item.w, c, false)
			}
		}
	}
}

// Text element and the inherited text-style patch.
module ui

import quadra.core
import quadra.prim
import quadra.layout
import quadra.text as txt

// TextPatch overrides parts of the inherited text style. `has` says which fields are set
// (bit flags instead of optionals: elements are built every frame, so they stay small).
pub struct TextPatch {
pub mut:
	has         u8
	family      string
	size        f32
	weight      int
	italic      bool
	color       prim.Color
	line_height f32
}

const tp_family = u8(1)
const tp_size = u8(2)
const tp_weight = u8(4)
const tp_italic = u8(8)
const tp_color = u8(16)
const tp_line_height = u8(32)

pub fn (mut p TextPatch) set_family(v string) {
	p.family, p.has = v, p.has | tp_family
}

pub fn (mut p TextPatch) set_size(v f32) {
	p.size, p.has = v, p.has | tp_size
}

pub fn (mut p TextPatch) set_weight(v int) {
	p.weight, p.has = v, p.has | tp_weight
}

pub fn (mut p TextPatch) set_italic(v bool) {
	p.italic, p.has = v, p.has | tp_italic
}

pub fn (mut p TextPatch) set_color(v prim.Color) {
	p.color, p.has = v, p.has | tp_color
}

pub fn (mut p TextPatch) set_line_height(v f32) {
	p.line_height, p.has = v, p.has | tp_line_height
}

pub fn (p &TextPatch) apply(base txt.TextStyle) txt.TextStyle {
	if p.has == 0 {
		return base
	}
	return txt.TextStyle{
		family:      if p.has & tp_family != 0 { p.family } else { base.family }
		size:        if p.has & tp_size != 0 { p.size } else { base.size }
		weight:      if p.has & tp_weight != 0 { p.weight } else { base.weight }
		italic:      if p.has & tp_italic != 0 { p.italic } else { base.italic }
		color:       if p.has & tp_color != 0 { p.color } else { base.color }
		line_height: if p.has & tp_line_height != 0 { p.line_height } else { base.line_height }
	}
}

// Text leaves use the default layout style: their size comes from measuring.
const text_leaf_style = layout.Style{}

@[heap]
pub struct TextEl {
pub mut:
	content string
	patch   TextPatch
mut:
	resolved txt.TextStyle
	ts       &txt.TextSystem  = unsafe { nil }
	wrapped  &txt.WrappedText = unsafe { nil } // the last measure: paint reuses it when it fits
	wrap_w   f32
	node     layout.NodeId
	select   bool // selectable.v
	sel      &SelectState = unsafe { nil }
	bounds   prim.Rect
}

pub fn text(s string) &TextEl {
	return &TextEl{
		content: s
	}
}

pub fn (self_ &TextEl) size(v f32) &TextEl {
	mut t := unsafe { self_ }
	t.patch.set_size(v)
	return t
}

pub fn (self_ &TextEl) color(c prim.Color) &TextEl {
	mut t := unsafe { self_ }
	t.patch.set_color(c)
	return t
}

pub fn (self_ &TextEl) weight(w int) &TextEl {
	mut t := unsafe { self_ }
	t.patch.set_weight(w)
	return t
}

pub fn (self_ &TextEl) bold() &TextEl {
	mut t := unsafe { self_ }
	t.patch.set_weight(700)
	return t
}

pub fn (self_ &TextEl) italic() &TextEl {
	mut t := unsafe { self_ }
	t.patch.set_italic(true)
	return t
}

pub fn (self_ &TextEl) family(f string) &TextEl {
	mut t := unsafe { self_ }
	t.patch.set_family(f)
	return t
}

pub fn (self_ &TextEl) line_height(v f32) &TextEl {
	mut t := unsafe { self_ }
	t.patch.set_line_height(v)
	return t
}

// fits reports whether the last wrap is also the wrap at max_w: true for any width from its
// widest line up to the width it was made for, or up without limit if no line broke for width.
fn (t &TextEl) fits(max_w f32) bool {
	return t.wrapped != unsafe { nil } && t.wrapped.width <= max_w
		&& (max_w <= t.wrap_w || !t.wrapped.width_breaks)
}

fn measure_text(ctx voidptr, kw f32, _ f32, aw f32, _ f32) prim.Size {
	mut t := unsafe { &TextEl(ctx) }
	limit := if kw != layout.unknown { kw } else { aw }
	max_w := if limit >= layout.unbounded { f32(1.0e9) } else { limit }
	if !t.fits(max_w) {
		t.wrapped, t.wrap_w = t.ts.wrap(t.content, t.resolved, max_w), max_w
	}
	w := t.wrapped
	return prim.size(if kw != layout.unknown { kw } else { w.width }, w.height)
}

pub fn (mut t TextEl) request_layout(mut cx core.LayoutCx) layout.NodeId {
	r := t.patch.apply(cx.text_style)
	if r != t.resolved {
		t.wrapped = unsafe { nil } // a cached element under a new inherited style
	}
	t.resolved = r
	t.ts = cx.text
	t.node = cx.tree.leaf(text_leaf_style, measure_text, voidptr(t))
	return t.node
}

pub fn (mut t TextEl) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.register_text(t.content, bounds)
	if t.select {
		t.prepaint_select(bounds, mut cx)
	}
}

pub fn (mut t TextEl) paint(bounds prim.Rect, mut cx core.PaintCx) {
	// A little slack so float rounding never re-wraps differently from layout.
	bw := bounds.width + 0.5
	w := if t.fits(bw) {
		t.wrapped
	} else {
		cx.text.wrap(t.content, t.resolved, bw)
	}
	asc, desc := cx.text.line_metrics(t.resolved)
	lead := (w.line_height - (asc + desc)) / 2
	if t.select {
		t.paint_selection(bounds, w.lines, w.line_height, mut cx)
	}
	// Lines outside the clip (a long text in a scrolled view) are not painted.
	clip := cx.clip_rect()
	for line in w.lines {
		top := bounds.y + line.y
		if top + w.line_height < clip.y - 2 || top > clip.y + clip.height + 2 {
			continue
		}
		cx.glyphs(line.layout, bounds.x, top + lead + asc, t.resolved.color)
	}
}

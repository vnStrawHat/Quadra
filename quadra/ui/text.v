// Text element and the inherited text-style patch.
module ui

import math
import strconv
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

const no_wrap = f32(1.0e9) // a width no line reaches

@[heap]
pub struct TextEl {
pub mut:
	content string
	patch   TextPatch
mut:
	resolved txt.TextStyle
	ts       &txt.TextSystem  = unsafe { nil }
	wrapped  &txt.WrappedText = unsafe { nil } // the last measure: paint reuses it when it fits
	own      &txt.WrappedText = unsafe { nil } // a number's wrap, refilled in place (kept by the arena)
	pooled   bool // from a render arena (arena.v)
	wrap_w   f32
	node     layout.NodeId
	one_line bool // truncate()
	select   bool // selectable.v
	sel      &SelectState = unsafe { nil }
	bounds   prim.Rect
}

// text makes a text element; like div(), it belongs to the render that made it.
pub fn text(s string) &TextEl {
	return new_text(s)
}

// number makes a text element showing v with `decimals` (0 to 9) digits after the point,
// exactly as '${v:.2f}' does but without a string builder: a table or a meter that changes
// every frame then allocates only its digits.
pub fn number(v f64, decimals int) &TextEl {
	return new_text(format_fixed(v, decimals))
}

fn format_fixed(v f64, decimals int) string {
	d := int(prim.clamp(f32(decimals), 0, 9))
	mut scale := u64(1)
	for _ in 0 .. d {
		scale *= 10
	}
	a := math.abs(v) * f64(scale)
	// Below 1e12 the product is off by less than 2.3e-4, so rounding it gives the exact
	// decimal rounding unless it is close to a tie. Ties, huge values, NaN and infinities
	// take strconv's exact path (the one interpolation uses).
	if !(a < 1.0e12) || math.abs(a - math.floor(a) - 0.5) < 1.0e-3 {
		return strconv.format_fl(v, strconv.BF_param{ len1: d, positive: !math.signbit(v) })
	}
	x := u64(math.round(a))
	mut buf := [32]u8{}
	mut p := 32
	mut ip := x / scale
	mut fp := x % scale
	if d > 0 {
		for _ in 0 .. d {
			p--
			buf[p] = u8(`0` + fp % 10)
			fp /= 10
		}
		p--
		buf[p] = `.`
	}
	for {
		p--
		buf[p] = u8(`0` + ip % 10)
		ip /= 10
		if ip == 0 {
			break
		}
	}
	if math.signbit(v) { // interpolation keeps the sign of -0.004 and -0.0
		p--
		buf[p] = `-`
	}
	return unsafe { tos(&buf[p], 32 - p) }.clone()
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

// truncate keeps the text on one line and ends it with … when it does not fit.
pub fn (self_ &TextEl) truncate() &TextEl {
	mut t := unsafe { self_ }
	t.one_line = true
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
	max_w := if t.one_line || limit >= layout.unbounded { no_wrap } else { limit }
	if !t.fits(max_w) {
		t.wrapped, t.wrap_w = t.wrap_at(max_w), max_w
	}
	w := t.wrapped
	width := if t.one_line { prim.min(w.width, limit) } else { w.width }
	return prim.size(if kw != layout.unknown { kw } else { width }, w.height)
}

// wrap_at wraps the text at max_w. An arena element lays a number out into its own wrap,
// which it keeps from render to render: no allocation and no cache entry. Other elements are
// new every time, so they use the cache, where a number that stays the same is found again.
fn (mut t TextEl) wrap_at(max_w f32) &txt.WrappedText {
	if t.pooled && !t.one_line {
		if w := t.ts.wrap_number(t.content, t.resolved, max_w, t.own) {
			t.own = w
			return w
		}
	}
	return t.ts.wrap(t.content, t.resolved, max_w)
}

// lines_at is what paint draws in a box w wide.
fn (t &TextEl) lines_at(w f32, mut cx core.PaintCx) &txt.WrappedText {
	if t.one_line {
		full := cx.text.wrap(t.content, t.resolved, no_wrap)
		if full.width <= w {
			return full
		}
		return cx.text.wrap(t.elided(w, mut cx), t.resolved, no_wrap)
	}
	return if t.fits(w) { t.wrapped } else { cx.text.wrap(t.content, t.resolved, w) }
}

// elided cuts the text at the last grapheme where it and … fit in w.
fn (t &TextEl) elided(w f32, mut cx core.PaintCx) string {
	room := w - cx.text.layout_line('…', t.resolved).width
	mut end := 0
	for stop in cx.text.layout_line(t.content, t.resolved).stops {
		if stop.x > room {
			break
		}
		end = stop.index
	}
	return t.content[..end].trim_right(' ') + '…'
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
	w := t.lines_at(bw, mut cx)
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

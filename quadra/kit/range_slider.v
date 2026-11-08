// Range slider: two thumbs on one track for a low and a high value. A press or drag moves
// the nearer thumb; each thumb takes focus and arrows on its own, never past the other.
module kit

import math
import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

pub struct Range {
pub:
	lo f32
	hi f32
}

@[heap]
pub struct RangeSlider {
	Lazy
pub mut:
	id     string
	value  Range
	min    f32
	max    f32 = 100
	step   f32 = 1
	width  f32 = 200
	change ?core.AnyListener
mut:
	lo_div &ui.Div = unsafe { nil }
	hi_div &ui.Div = unsafe { nil }
}

pub fn range_slider(id string, lo f32, hi f32) &RangeSlider {
	return &RangeSlider{
		id:    id
		value: Range{lo, hi}
	}
}

pub fn (self_ &RangeSlider) range(min f32, max f32, step f32) &RangeSlider {
	mut s := unsafe { self_ }
	s.min, s.max, s.step = min, max, step
	return s
}

pub fn (self_ &RangeSlider) width(w f32) &RangeSlider {
	mut s := unsafe { self_ }
	s.width = w
	return s
}

pub fn (self_ &RangeSlider) on_change(l core.Listener[Range]) &RangeSlider {
	mut s := unsafe { self_ }
	s.change = l.any
	return s
}

// nearer moves the thumb nearer to v there; on a tie, the one on v's side.
fn (r Range) nearer(v f32) Range {
	dl, dh := math.abs(v - r.lo), math.abs(v - r.hi)
	return if dl < dh || (dl == dh && v < r.lo) { Range{v, r.hi} } else { Range{r.lo, v} }
}

// RangeData is what the slider's handlers need (closure-free handlers, specs/14).
struct RangeData {
	spec SliderSpec
	r    Range
}

// range_thumb is a 12 px knob like the slider's (specs/15).
fn range_thumb(id string, label string, value f32, x f32, t &Theme) &ui.Div {
	return ui.div().test_id(id).absolute().left(x + 2).top(4).size(12).rounded_full().bg(t.p.text).border(1).border_color(t.p.canvas).role(.slider).label(label).sem_value('${value}')
}

fn set_range(r Range, old Range, change ?core.AnyListener, mut cx core.Cx) {
	if r != old {
		fire(change, r, mut cx)
	}
}

// thumb_key handles a focused thumb's arrows (low is true for the low thumb).
fn thumb_key(spec SliderSpec, r Range, low bool, e &core.KeyDownEvent, mut cx core.Cx) {
	big := (spec.max - spec.min) / 10
	at := if low { r.lo } else { r.hi }
	v := match e.key {
		.right, .up { spec.snap(at + spec.step) }
		.left, .down { spec.snap(at - spec.step) }
		.page_up { spec.snap(at + big) }
		.page_down { spec.snap(at - big) }
		.home { spec.min }
		.end { spec.max }
		else { return }
	}

	n := if low { Range{prim.min(v, r.hi), r.hi} } else { Range{r.lo, prim.max(v, r.lo)} }
	set_range(n, r, spec.change, mut cx)
	cx.stop_propagation()
}

pub fn (mut s RangeSlider) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut s, mut cx)
	return s.layout_inner(el, mut cx)
}

pub fn (mut s RangeSlider) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	p := t.p
	r := s.value
	spec := SliderSpec{s.min, s.max, s.step, 0, s.change}
	span := prim.max(s.max - s.min, 0.0001)
	track_w := s.width - 16
	fl := prim.clamp((r.lo - s.min) / span, 0, 1)
	fh := prim.clamp((r.hi - s.min) / span, 0, 1)
	data := RangeData{spec, r}
	mut lo := range_thumb('${s.id}-lo', 'Minimum', r.lo, track_w * fl, t)
	lo.on_key_down(core.handler_with(data, fn (d RangeData, e &core.KeyDownEvent, mut cx core.Cx) {
		thumb_key(d.spec, d.r, true, e, mut cx)
	}))
	mut hi := range_thumb('${s.id}-hi', 'Maximum', r.hi, track_w * fh, t)
	hi.on_key_down(core.handler_with(data, fn (d RangeData, e &core.KeyDownEvent, mut cx core.Cx) {
		thumb_key(d.spec, d.r, false, e, mut cx)
	}))
	s.lo_div, s.hi_div = lo, hi
	mut d := ui.div().id(s.id).test_id(s.id).w(s.width).h(20).role(.group).label(s.id).cursor(.pointer).child(ui.div().absolute().left(8).top(8).w(track_w).h(4).rounded_full().bg(p.press)).child(ui.div().absolute().left(
		8 + track_w * fl).top(8).w(track_w * (fh - fl)).h(4).rounded_full().bg(p.accent)).child(lo).child(hi)

	d.on_mouse_down(core.handler_with(data, fn (d RangeData, e &core.MouseDownEvent, mut cx core.Cx) {
		cx.capture_mouse()
		set_range(d.r.nearer(d.spec.at_x(e.x, cx.target_bounds())), d.r, d.spec.change, mut cx)
	})).on_mouse_move(core.handler_with(data, fn (d RangeData, e &core.MouseMoveEvent, mut cx core.Cx) {
		if e.pressed {
			set_range(d.r.nearer(d.spec.at_x(e.x, cx.target_bounds())), d.r, d.spec.change, mut cx)
		}
	}))
	return d
}

pub fn (mut s RangeSlider) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	s.lo_div.focus_id = stable_focus(mut cx, '${s.id}-lo')
	s.hi_div.focus_id = stable_focus(mut cx, '${s.id}-hi')
	s.Lazy.prepaint(bounds, mut cx)
}

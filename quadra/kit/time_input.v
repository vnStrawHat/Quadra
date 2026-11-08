// Time input: hours and minutes as two focusable segments. Up and down step the focused
// one (wrapping), digits type it (two in a row make a two-digit value), tab moves on.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.platform
import quadra.ui

pub struct TimeOfDay {
pub:
	hour   int // 0..23
	minute int // 0..59
}

pub fn (t TimeOfDay) str() string {
	return '${t.hour:02}:${t.minute:02}'
}

@[heap]
pub struct TimeInput {
	Lazy
pub mut:
	id     string
	value  TimeOfDay
	step   int = 1 // minutes per up/down
	change ?core.AnyListener
mut:
	hour_div &ui.Div = unsafe { nil }
	min_div  &ui.Div = unsafe { nil }
}

pub fn time_input(id string, value TimeOfDay) &TimeInput {
	return &TimeInput{
		id:    id
		value: value
	}
}

pub fn (self_ &TimeInput) minute_step(n int) &TimeInput {
	mut t := unsafe { self_ }
	t.step = n
	return t
}

pub fn (self_ &TimeInput) on_change(l core.Listener[TimeOfDay]) &TimeInput {
	mut t := unsafe { self_ }
	t.change = l.any
	return t
}

fn digit_of(k platform.Key) int {
	n := int(k) - int(platform.Key.digit0)
	return if n >= 0 && n <= 9 { n } else { -1 }
}

// typed is a segment after typing digit d: appended when that still fits under limit
// (typing 1 then 5 gives 15), else d alone.
fn typed(old int, d int, limit int) int {
	two := (old % 10) * 10 + d
	return if old < 10 && two < limit { two } else { d }
}

// segment_key is a segment's new value for a key, or -1 when the key is not for it.
fn segment_key(e &core.KeyDownEvent, old int, step int, limit int) int {
	d := digit_of(e.key)
	if d >= 0 {
		return typed(old, d, limit)
	}
	return match e.key {
		.up { (old + step) % limit }
		.down { (old - step % limit + limit) % limit }
		.home { 0 }
		.end { limit - 1 }
		else { -1 }
	}
}

fn time_segment(id string, text string, label string, t &Theme) &ui.Div {
	return ui.div().test_id(id).px(3).rounded(t.radius_sm).role(.slider).label(label).sem_value(text).focus_ring(ui.Patch{
		background: t.p.press
	}).child(ui.text(text))
}

pub fn (mut ti TimeInput) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut ti, mut cx)
	return ti.layout_inner(el, mut cx)
}

// TimeData is what the segments' key handlers need (closure-free handlers, specs/14).
struct TimeData {
	v      TimeOfDay
	step   int
	change ?core.AnyListener
}

pub fn (mut ti TimeInput) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	p := t.p
	v, step, change := ti.value, ti.step, ti.change
	mut h := time_segment('${ti.id}-h', '${v.hour:02}', 'Hours', t)
	data := TimeData{v, step, change}
	h.on_key_down(core.handler_with(data, fn (d TimeData, e &core.KeyDownEvent, mut cx core.Cx) {
		v, change := d.v, d.change
		n := segment_key(e, v.hour, 1, 24)
		if n >= 0 {
			fire(change, TimeOfDay{n, v.minute}, mut cx)
			cx.stop_propagation()
		}
	}))
	mut m := time_segment('${ti.id}-m', '${v.minute:02}', 'Minutes', t)
	m.on_key_down(core.handler_with(data, fn (d TimeData, e &core.KeyDownEvent, mut cx core.Cx) {
		v, step, change := d.v, d.step, d.change
		n := segment_key(e, v.minute, if e.key in [.up, .down] { step } else { 1 }, 60)
		if n >= 0 {
			fire(change, TimeOfDay{v.hour, n}, mut cx)
			cx.stop_propagation()
		}
	}))
	ti.hour_div, ti.min_div = h, m
	return ui.div().id(ti.id).test_id(ti.id).flex_row().items_center().gap(6).w(120).h(t.height(.md)).px(8).rounded(t.radius_md).border(1).border_color(p.line).role(.group).label(v.str()).child(icon(icon_clock).size(14).color(p.text_2)).child(ui.div().flex_row().items_center().child(h).child(ui.text(':')).child(m))
}

pub fn (mut ti TimeInput) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	ti.hour_div.focus_id = stable_focus(mut cx, '${ti.id}-h')
	ti.min_div.focus_id = stable_focus(mut cx, '${ti.id}-m')
	ti.Lazy.prepaint(bounds, mut cx)
}

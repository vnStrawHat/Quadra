// Slider: drag the thumb or click the track; arrows and home/end when focused.
module kit

import math
import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

@[heap]
pub struct Slider {
	Lazy
pub mut:
	id       string
	value    f32
	min      f32
	max      f32 = 100
	step     f32 = 1
	width    f32 = 200
	disabled bool
	change   ?core.AnyListener
mut:
	div &ui.Div = unsafe { nil }
}

pub fn slider(id string, value f32) &Slider {
	return &Slider{
		id:    id
		value: value
	}
}

pub fn (self_ &Slider) range(min f32, max f32, step f32) &Slider {
	mut s := unsafe { self_ }
	s.min, s.max, s.step = min, max, step
	return s
}

pub fn (self_ &Slider) width(w f32) &Slider {
	mut s := unsafe { self_ }
	s.width = w
	return s
}

pub fn (self_ &Slider) disabled(on bool) &Slider {
	mut s := unsafe { self_ }
	s.disabled = on
	return s
}

pub fn (self_ &Slider) on_change(l core.Listener[f32]) &Slider {
	mut s := unsafe { self_ }
	s.change = l.any
	return s
}

struct SliderSpec {
	min    f32
	max    f32
	step   f32
	value  f32
	change ?core.AnyListener
}

fn (s SliderSpec) snap(v f32) f32 {
	stepped := if s.step > 0 { f32(math.round((v - s.min) / s.step)) * s.step + s.min } else { v }
	return prim.clamp(stepped, s.min, s.max)
}

fn (s SliderSpec) at_x(x f32, bounds prim.Rect) f32 {
	t := prim.clamp((x - bounds.x - 8) / prim.max(1, bounds.width - 16), 0, 1)
	return s.snap(s.min + t * (s.max - s.min))
}

fn (s SliderSpec) set(v f32, mut cx core.Cx) {
	if v != s.value {
		fire(s.change, v, mut cx)
	}
}

pub fn (mut s Slider) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut s, mut cx)
	return s.layout_inner(el, mut cx)
}

pub fn (mut s Slider) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	p := t.p
	frac := if s.max > s.min { prim.clamp((s.value - s.min) / (s.max - s.min), 0, 1) } else { 0 }
	track_w := s.width - 16
	spec := SliderSpec{s.min, s.max, s.step, s.value, s.change}
	// Derived from the switch and the progress bar (specs/15): a 4 px track, the accent
	// fill, a 12 px knob in the text color.
	fill := if s.disabled { p.text_disabled } else { p.accent }
	mut d := ui.div().id(s.id).test_id(s.id).w(s.width).h(20).role(.slider).label(s.id).sem_value('${s.value}').disabled(s.disabled).child(ui.div().absolute().left(8).top(8).w(track_w).h(4).rounded_full().bg(p.press)).child(ui.div().absolute().left(8).top(8).w(track_w * frac).h(4).rounded_full().bg(fill)).child(ui.div().absolute().left(
		track_w * frac + 2).top(4).size(12).rounded_full().bg(if s.disabled {
		p.text_disabled
	} else {
		p.text
	}).border(1).border_color(p.canvas))
	if !s.disabled {
		d.cursor(.pointer).on_mouse_down(core.handler_with(spec, fn (spec SliderSpec, e &core.MouseDownEvent, mut cx core.Cx) {
			cx.capture_mouse()
			spec.set(spec.at_x(e.x, cx.target_bounds()), mut cx)
		})).on_mouse_move(core.handler_with(spec, fn (spec SliderSpec, e &core.MouseMoveEvent, mut cx core.Cx) {
			if e.pressed {
				spec.set(spec.at_x(e.x, cx.target_bounds()), mut cx)
			}
		})).on_key_down(core.handler_with(spec, fn (spec SliderSpec, e &core.KeyDownEvent, mut cx core.Cx) {
			big := (spec.max - spec.min) / 10
			v := match e.key {
				.right, .up { spec.snap(spec.value + spec.step) }
				.left, .down { spec.snap(spec.value - spec.step) }
				.page_up { spec.snap(spec.value + big) }
				.page_down { spec.snap(spec.value - big) }
				.home { spec.min }
				.end { spec.max }
				else { return }
			}

			spec.set(v, mut cx)
			cx.stop_propagation()
		}))
	}
	s.div = d
	return d
}

pub fn (mut s Slider) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	if !s.disabled {
		s.div.focus_id = stable_focus(mut cx, s.id)
	}
	s.Lazy.prepaint(bounds, mut cx)
}

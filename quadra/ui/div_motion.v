// Div transforms and transitions. translate, scale and rotate move what a div paints (its
// children too) without touching layout or hit testing. transition(ms) eases background,
// border color, opacity and the transforms towards new values, whether they come from a
// hover or press patch or from a new render. Timelines are in div_timeline.v.
module ui

import quadra.core
import quadra.prim

// translate paints the div moved by (x, y) logical px.
pub fn (self_ &Div) translate(x f32, y f32) &Div {
	mut d := unsafe { self_ }
	d.visual.translate = prim.Point{x, y}
	return d
}

// scale paints the div scaled by k about its center.
pub fn (self_ &Div) scale(k f32) &Div {
	mut d := unsafe { self_ }
	d.visual.scale = k
	return d
}

// rotate paints the div turned by deg degrees (clockwise) about its center.
pub fn (self_ &Div) rotate(deg f32) &Div {
	mut d := unsafe { self_ }
	d.visual.rotate = deg
	return d
}

// transition eases visual changes over ms milliseconds.
pub fn (self_ &Div) transition(ms f32) &Div {
	return unsafe { self_ }.transition_with(ms, .ease_out)
}

pub fn (self_ &Div) transition_with(ms f32, easing core.Easing) &Div {
	mut d := unsafe { self_ }
	mut st := d.states_mut()
	st.transition = ms
	st.easing = easing
	return d
}

// Tween is the part of a Visual a transition eases.
struct Tween {
	background   prim.Color
	border_color prim.Color
	opacity      f32
	translate    prim.Point
	scale        f32
	rotate       f32
}

fn tween_of(v Visual) Tween {
	return Tween{v.background, v.border_color, v.opacity, v.translate, v.scale, v.rotate}
}

// with_tween is v showing t.
fn with_tween(v Visual, t Tween) Visual {
	return Visual{
		...v
		background:   t.background
		border_color: t.border_color
		opacity:      t.opacity
		translate:    t.translate
		scale:        t.scale
		rotate:       t.rotate
	}
}

fn mix_color(a prim.Color, b prim.Color, t f32) prim.Color {
	return prim.Color{a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a +
		(b.a - a.a) * t}
}

fn mix(a Tween, b Tween, t f32) Tween {
	return Tween{
		background:   mix_color(a.background, b.background, t)
		border_color: mix_color(a.border_color, b.border_color, t)
		opacity:      a.opacity + (b.opacity - a.opacity) * t
		translate:    prim.Point{a.translate.x + (b.translate.x - a.translate.x) * t,
			a.translate.y + (b.translate.y - a.translate.y) * t}
		scale:        a.scale + (b.scale - a.scale) * t
		rotate:       a.rotate + (b.rotate - a.rotate) * t
	}
}

// Transition is a div's eased state, kept in element state across frames.
pub struct Transition {
mut:
	ready bool
	from  Tween
	to    Tween
	start f64
}

fn (t &Transition) at(now f64, ms f32, e core.Easing) Tween {
	if ms <= 0 || now >= t.start + ms {
		return t.to
	}
	return mix(t.from, t.to, core.eased(e, f32((now - t.start) / ms)))
}

fn (mut d Div) prepaint_transition(mut cx core.PrepaintCx) {
	if d.states.transition > 0 {
		cx.enter('transition')
		d.states.tr = cx.state(Transition{})
		cx.leave()
	}
	if d.states.motion != unsafe { nil } {
		cx.enter('motion')
		d.states.clock = cx.state(Clock{})
		cx.leave()
	}
}

// transitioned is v with the eased values of now; a change of target starts from where
// the last one was. The first paint shows the target at once.
fn (d &Div) transitioned(v Visual, mut cx core.PaintCx) Visual {
	mut st := d.states.tr
	target := tween_of(v)
	now := cx.now_ms()
	ms, e := d.states.transition, d.states.easing
	if !st.ready {
		st.ready = true
		st.from, st.to, st.start = target, target, now - ms
	} else if target != st.to {
		st.from, st.start = st.at(now, ms, e), now
		st.to = target
	}
	if now < st.start + ms {
		cx.request_paint_after(16)
	}
	return with_tween(v, st.at(now, ms, e))
}

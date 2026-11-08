// Animations driven by the frame clock (virtual in headless tests).
module core

import math

pub enum Easing {
	linear
	ease_in
	ease_out
	ease_in_out
}

struct Anim {
mut:
	from      f32
	to        f32
	start     f64
	duration  f64
	easing    Easing
	view      u32
	last_tick f64
}

// eased maps progress t in [0, 1] through the easing curve.
pub fn eased(e Easing, t f32) f32 {
	return ease(e, t)
}

fn ease(e Easing, t f32) f32 {
	return match e {
		.linear {
			t
		}
		.ease_in {
			t * t * t
		}
		.ease_out {
			1 - (1 - t) * (1 - t) * (1 - t)
		}
		.ease_in_out {
			if t < 0.5 {
				4 * t * t * t
			} else {
				1 - f32(math.pow(-2 * t + 2, 3)) / 2
			}
		}
	}
}

fn (a &Anim) value(now f64) f32 {
	if a.duration <= 0 || now >= a.start + a.duration {
		return a.to
	}
	t := f32((now - a.start) / a.duration)
	return a.from + (a.to - a.from) * ease(a.easing, t)
}

// animate returns the current value of an animation towards `target`. Changing the
// target restarts from the current value. The view keeps rendering until it settles.
pub fn (mut cx Cx) animate(key string, target f32, duration_ms f64, easing Easing) f32 {
	k := '${cx.entity}:${key}'
	app := cx.app
	now := app.plat.now_ms()
	mut a := cx.app.anims[k] or {
		cx.app.anims[k] = Anim{
			from:  target
			to:    target
			view:  cx.entity
			start: now
		}
		return target
	}
	if a.to != target {
		a = Anim{
			from:     a.value(now)
			to:       target
			start:    now
			duration: duration_ms
			easing:   easing
			view:     cx.entity
		}
		cx.app.anims[k] = a
	}
	if now < a.start + a.duration {
		cx.app.animating[cx.entity] = true
	}
	return a.value(now)
}

// tick_animations keeps running animations moving. On a real display the next vsync
// frame re-renders them; headless, a 16 ms timer on the virtual clock does, so tests
// advance animations with harness.advance().
fn (mut app App) tick_animations(mut _ Window) {
	if app.animating.len == 0 || app.anim_timer != 0 {
		return
	}
	if app.plat.is_headless() {
		app.anim_timer = app.plat.set_timer(16, false)
		return
	}
	app.advance_animations()
}

fn (mut app App) advance_animations() {
	app.anim_timer = 0
	views := app.animating.keys()
	app.animating.clear()
	for v in views {
		if int(v) < app.store.slots.len && app.store.slots[v].alive {
			app.changed(v)
		}
	}
	app.flush()
}

pub fn (app &App) is_animating() bool {
	return app.animating.len > 0
}

// request_animation_frame renders the current view again at the next frame (once per
// call): a view that changes every frame calls it from render.
pub fn (mut cx Cx) request_animation_frame() {
	cx.app.animating[cx.entity] = true
}

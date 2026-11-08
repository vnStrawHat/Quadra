// Animation timelines: div.animate(m) plays m from the div's first paint, on top of its
// style and transition (specs/08, Motion). A timeline is a tree of steps, sequences and
// parallel groups; it is evaluated afresh at each paint from the time since the start.
module ui

import math
import quadra.core

enum MotionKind as u8 {
	step
	seq
	par
}

// Motion is a timeline: build it with tween, tween_with and pause, then `+`, parallel,
// repeat, forever and alternate.
pub struct Motion {
	kind      MotionKind
	ms        f32
	easing    core.Easing
	to        Patch
	parts     []Motion
	count     int = 1 // plays; 0 = forever
	alternate bool
}

// tween eases the properties `to` sets (background, border color, opacity, translate,
// scale, rotate) from where they are to its values over ms milliseconds.
pub fn tween(ms f32, to Patch) Motion {
	return tween_with(ms, .ease_in_out, to)
}

pub fn tween_with(ms f32, easing core.Easing, to Patch) Motion {
	return Motion{
		ms:     ms
		easing: easing
		to:     to
	}
}

// pause waits ms milliseconds.
pub fn pause(ms f32) Motion {
	return tween_with(ms, .linear, Patch{})
}

// a + b plays b once a is over.
pub fn (a Motion) + (b Motion) Motion {
	if a.kind == .seq && a.count == 1 && !a.alternate {
		mut parts := a.parts.clone()
		parts << b
		return Motion{
			kind:  .seq
			parts: parts
		}
	}
	return Motion{
		kind:  .seq
		parts: [a, b]
	}
}

// parallel plays the motions together; a later one wins a property both set.
pub fn parallel(motions ...Motion) Motion {
	return Motion{
		kind:  .par
		parts: motions
	}
}

// repeat plays the motion n times, each from the values it started from.
pub fn (m Motion) repeat(n int) Motion {
	return Motion{
		...m
		count: if n < 1 { 1 } else { n }
	}
}

// forever repeats the motion until the div goes away.
pub fn (m Motion) forever() Motion {
	return Motion{
		...m
		count: 0
	}
}

// alternate plays every second repeat backwards.
pub fn (m Motion) alternate() Motion {
	return Motion{
		...m
		alternate: true
	}
}

// once is the length of one play.
fn (m &Motion) once() f32 {
	mut d := f32(0)
	match m.kind {
		.step {
			d = m.ms
		}
		.seq {
			for p in m.parts {
				d += p.total()
			}
		}
		.par {
			for p in m.parts {
				d = f32(math.max(d, p.total()))
			}
		}
	}

	return d
}

// total is the length with the repeats (infinite for forever).
pub fn (m &Motion) total() f32 {
	if m.count == 0 {
		return f32(math.inf(1))
	}
	return m.once() * m.count
}

// local maps time since the start to time within one play.
fn (m &Motion) local(e f32) f32 {
	one := m.once()
	if one <= 0 {
		return 0
	}
	if m.count != 0 && e >= one * m.count {
		return if m.alternate && m.count % 2 == 0 { 0 } else { one }
	}
	cycle := int(e / one)
	r := e - one * cycle
	return if m.alternate && cycle % 2 == 1 { one - r } else { r }
}

// at is the tween e ms after the start, from cur.
fn (m &Motion) at(e f32, cur Tween) Tween {
	r := m.local(e)
	match m.kind {
		.step {
			if m.ms <= 0 || r >= m.ms {
				return patched(cur, m.to)
			}
			return mix(cur, patched(cur, m.to), core.eased(m.easing, r / m.ms))
		}
		.seq {
			mut c := cur
			mut offset := f32(0)
			for p in m.parts {
				d := p.total()
				if r < offset + d {
					return p.at(r - offset, c)
				}
				c = p.at(d, c)
				offset += d
			}
			return c
		}
		.par {
			mut c := cur
			for p in m.parts {
				c = p.at(r, c)
			}
			return c
		}
	}
}

fn patched(t Tween, p Patch) Tween {
	mut v := with_tween(Visual{}, t)
	mut tp := TextPatch{}
	apply_patch(mut v, mut tp, p)
	return tween_of(v)
}

// Clock is when a div's timeline started, kept in element state.
pub struct Clock {
mut:
	started bool
	start   f64
}

// animate plays m from the first paint; the end values stay. Changing the div's id
// restarts it.
pub fn (self_ &Div) animate(m Motion) &Div {
	mut d := unsafe { self_ }
	d.states_mut().motion = &Motion{
		...m
	}
	return d
}

fn (d &Div) animated(v Visual, mut cx core.PaintCx) Visual {
	mut c := d.states.clock
	now := cx.now_ms()
	if !c.started {
		c.started, c.start = true, now
	}
	e := f32(now - c.start)
	m := d.states.motion
	if e < m.total() {
		cx.request_paint_after(16)
	}
	return with_tween(v, m.at(e, tween_of(v)))
}

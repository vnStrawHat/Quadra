// Elements from render arenas (core/arena.v, specs/07): during a view's render, div() and
// text() hand out elements of the view's render before last, reset; elsewhere they allocate.
module ui

import quadra.core
import quadra.prim

const arena_divs = 0
const arena_texts = 1
const arena_svgs = 2

fn init() {
	core.set_arena_reset(reset_arena)
}

// new_div is div() from the current render's arena, or the heap.
@[inline]
fn new_div() &Div {
	mut a := core.render_arena()
	if a == unsafe { nil } {
		return &Div{}
	}
	k := a.used[arena_divs]
	a.used[arena_divs] = k + 1
	if k < a.pools[arena_divs].len {
		return prim.held[Div](a.pools[arena_divs][k])
	}
	d := &Div{}
	a.pools[arena_divs] << prim.hold(d)
	return d
}

@[inline]
fn new_text(s string) &TextEl {
	mut a := core.render_arena()
	if a == unsafe { nil } {
		return &TextEl{
			content: s
		}
	}
	k := a.used[arena_texts]
	a.used[arena_texts] = k + 1
	if k < a.pools[arena_texts].len {
		mut t := prim.held[TextEl](a.pools[arena_texts][k])
		t.content = s
		return t
	}
	t := &TextEl{
		content: s
		pooled:  true
	}
	a.pools[arena_texts] << prim.hold(t)
	return t
}

@[inline]
fn new_svg(src string, size f32) &SvgEl {
	mut a := core.render_arena()
	if a == unsafe { nil } {
		return &SvgEl{
			src: src
			w:   size
			h:   size
		}
	}
	k := a.used[arena_svgs]
	a.used[arena_svgs] = k + 1
	if k < a.pools[arena_svgs].len {
		mut s := prim.held[SvgEl](a.pools[arena_svgs][k])
		same := s.src.len == src.len && (unsafe { s.src.str == src.str } || s.src == src)
		if !same {
			s.parsed, s.hash, s.key = none, 0, ''
		}
		s.src, s.w, s.h = src, size, size
		return s
	}
	s := &SvgEl{
		src: src
		w:   size
		h:   size
	}
	a.pools[arena_svgs] << prim.hold(s)
	return s
}

// reset_arena puts the elements a render took back to their defaults. A div keeps the
// capacity of its children, handler and key arrays (cleared: it owns them, builders only
// append) and its state patches, reset, for the next states_mut().
fn reset_arena(mut a core.Arena) {
	for i in 0 .. a.used[arena_divs] {
		mut d := prim.held[Div](a.pools[arena_divs][i])
		mut kids := unsafe { d.children }
		unsafe { vmemset(kids.data, 0, kids.len * int(sizeof(core.Element))) }
		kids.clear()
		mut hs := unsafe { d.handlers }
		unsafe { vmemset(hs.data, 0, hs.len * int(sizeof(Handler))) }
		hs.clear()
		mut ks := unsafe { d.keys }
		unsafe { vmemset(ks.data, 0, ks.len * int(sizeof(KeySpec))) }
		ks.clear()
		mut spare := if d.states != unsafe { nil } { d.states } else { d.spare }
		if spare != unsafe { nil } {
			unsafe {
				*spare = States{}
			}
		}
		unsafe {
			*d = Div{}
		}
		d.children, d.handlers, d.keys, d.spare = kids, hs, ks, spare
	}
	for i in 0 .. a.used[arena_texts] {
		mut t := prim.held[TextEl](a.pools[arena_texts][i])
		own := t.own // a number's wrap buffers, refilled by the next use
		unsafe {
			*t = TextEl{}
		}
		t.own, t.pooled = own, true
	}
	for i in 0 .. a.used[arena_svgs] {
		mut s := prim.held[SvgEl](a.pools[arena_svgs][i])
		// the source and what depends on it only: new_svg compares the next source
		src, parsed, hash, key, kw, kh, kc := s.src, s.parsed, s.hash, s.key, s.kw, s.kh, s.kcolor
		unsafe {
			*s = SvgEl{}
		}
		s.src, s.parsed, s.hash, s.key, s.kw, s.kh, s.kcolor = src, parsed, hash, key, kw, kh, kc
	}
}

// Elements from render arenas (core/arena.v, specs/07): during a view's render, div() and
// text() hand out elements of the view's render before last, reset; elsewhere they allocate.
module ui

import quadra.core

const arena_divs = 0
const arena_texts = 1

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
		return unsafe { &Div(a.pools[arena_divs][k]) }
	}
	d := &Div{}
	a.pools[arena_divs] << voidptr(d)
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
		mut t := unsafe { &TextEl(a.pools[arena_texts][k]) }
		t.content = s
		return t
	}
	t := &TextEl{
		content: s
		pooled:  true
	}
	a.pools[arena_texts] << voidptr(t)
	return t
}

// reset_arena puts the elements a render took back to their defaults. A div keeps its
// children array's capacity (cleared): it always owns that array (child() and children()
// copy into it).
fn reset_arena(mut a core.Arena) {
	for i in 0 .. a.used[arena_divs] {
		mut d := unsafe { &Div(a.pools[arena_divs][i]) }
		mut kids := unsafe { d.children }
		unsafe { vmemset(kids.data, 0, kids.len * int(sizeof(core.Element))) }
		kids.clear()
		unsafe {
			*d = Div{}
		}
		d.children = kids
	}
	for i in 0 .. a.used[arena_texts] {
		mut t := unsafe { &TextEl(a.pools[arena_texts][i]) }
		own := t.own // a number's wrap buffers, refilled by the next use
		unsafe {
			*t = TextEl{}
		}
		t.own, t.pooled = own, true
	}
}

// Checked holders (specs/14): a pointer read back as its own type comes back unchanged;
// outside -prod another type panics with both names; in -prod a holder is the bare pointer.
module main

import os
import quadra.core
import quadra.prim
import quadra.testing
import quadra.ui

struct Apple {
mut:
	seeds int
}

struct Brick {
	weight int
}

fn test_a_matching_read_returns_the_pointer() {
	mut a := &Apple{3}
	h := prim.hold(a)
	mut back := prim.held[Apple](h)
	assert voidptr(back) == voidptr(a)
	back.seeds = 7
	assert a.seeds == 7
	assert prim.holds[Apple](h)
	assert prim.held[Brick](prim.Holder{}) == unsafe { nil } // empty: nil, never a panic
}

fn test_a_mismatch_is_reported_outside_prod() {
	h := prim.hold(&Apple{3})
	$if prod {
		assert prim.holds[Brick](h) // no tag in release builds
	} $else {
		assert !prim.holds[Brick](h)
	}
}

fn test_a_mismatched_read_panics_with_both_names() {
	$if prod {
		return
	}
	dir := os.join_path(os.temp_dir(), 'quadra-tests')
	os.mkdir_all(dir)!
	exe := os.join_path(dir, 'holder_child.exe')
	src := os.join_path(os.dir(@FILE), 'holder_child')
	b := os.execute('${os.quoted_path(@VEXE)} -o ${os.quoted_path(exe)} ${os.quoted_path(src)}')
	assert b.exit_code == 0, b.output
	r := os.execute(os.quoted_path(exe))
	assert r.exit_code != 0, r.output
	assert r.output.contains('a holder of Apple was read as Brick'), r.output
	assert !r.output.contains('no panic')
}

fn test_release_holders_are_bare_pointers() {
	$if prod {
		assert sizeof(prim.Holder) == sizeof(voidptr)
		assert sizeof(core.AnyListener) == 3 * sizeof(voidptr) // view (padded), holder, call
	} $else {
		assert sizeof(prim.Holder) > sizeof(voidptr) // the tag
	}
}

struct Knob {
mut:
	turns int
}

@[markused]
pub fn (mut k Knob) render(mut cx core.ViewCx[Knob]) core.Element {
	return ui.text('${k.turns}')
}

fn turn(mut k Knob, e &core.ClickEvent, mut cx core.ViewCx[Knob]) {
	k.turns += e.clicks
}

fn test_listeners_read_their_view_and_event_through_holders() {
	mut h := testing.new_harness()!
	v := h.open(Knob{})!
	mut cx := core.Cx{
		app: h.app
		win: h.window()
	}
	l := core.listener_of[Knob, core.ClickEvent](v.id, turn)
	ev := core.ClickEvent{
		clicks: 2
	}
	cx.app.invoke(l.any, &ev, cx.win, true)
	assert cx.read_view(v).turns == 2
}

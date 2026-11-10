// Div builder: scrolling (overflow, app-owned scroll positions, the wheel handler).
module ui

import quadra.core
import quadra.prim

// clip_box is what a clipping Div clips to: an axis it leaves visible keeps the parent's
// extent, so a ring beside a vertical scroller still shows (specs/08, Clipping).
fn (d &Div) clip_box(bounds prim.Rect, parent prim.Rect) prim.Rect {
	mut r := bounds
	if d.style.overflow_x == .visible {
		r.x, r.width = parent.x, parent.width
	}
	if d.style.overflow_y == .visible {
		r.y, r.height = parent.y, parent.height
	}
	return r
}

// ScrollState is kept in element state, keyed by the Div's id path.
pub struct ScrollState {
pub mut:
	x f32
	y f32
}

struct ScrollTarget {
mut:
	state   &ScrollState = unsafe { nil }
	max_x   f32
	max_y   f32
	win     &core.Window = unsafe { nil }
	wheel_x bool
}

fn scroll_handler(holder voidptr, _ voidptr, event voidptr, mut cx core.Cx) {
	mut t := unsafe { &ScrollTarget(holder) }
	e := unsafe { &core.ScrollEvent(event) }
	// Shift+wheel scrolls horizontally on mice without a horizontal wheel; so does the plain
	// wheel where the Div opted in (wheel_x).
	sideways := e.mods.has(.shift) || t.wheel_x
	dx := if e.dx == 0 && sideways { e.dy } else { e.dx }
	dy := if sideways { f32(0) } else { e.dy }
	nx := prim.clamp(t.state.x - dx, 0, t.max_x)
	ny := prim.clamp(t.state.y - dy, 0, t.max_y)
	if nx != t.state.x || ny != t.state.y {
		t.state.x, t.state.y = nx, ny
		cx.win.invalidate_prepaint()
		cx.stop_propagation() // consumed: outer scroll containers stay put
	}
}

// paint_thumbs draws the auto-hiding thumbs of a scrolling Div (core scroll_thumb). Strips
// that the plain wheel scrolls sideways (tab strips) show none.
fn (d &Div) paint_thumbs(bounds prim.Rect, mut cx core.PaintCx) {
	if d.wheel_x || (d.style.overflow_x != .scroll && d.style.overflow_y != .scroll) {
		return
	}
	content := cx.win.tree.content_size(d.node)
	target := core.ThumbTarget{voidptr(d.scroll_target.state), set_div_scroll}
	if d.style.overflow_y == .scroll {
		cx.scroll_thumb(bounds, content.height, d.scroll.y, true, target)
	}
	if d.style.overflow_x == .scroll {
		cx.scroll_thumb(bounds, content.width, d.scroll.x, false, target)
	}
}

fn set_div_scroll(holder voidptr, offset f32, vertical bool) {
	mut st := unsafe { &ScrollState(holder) }
	if vertical {
		st.y = offset
	} else {
		st.x = offset
	}
}

// overflow_y_scroll scrolls and clips vertically; the sides stay visible (specs/08, Clipping).
pub fn (self_ &Div) overflow_y_scroll() &Div {
	mut d := unsafe { self_ }
	d.style.overflow_y = .scroll
	return d
}

pub fn (self_ &Div) overflow_x_scroll() &Div {
	mut d := unsafe { self_ }
	d.style.overflow_x = .scroll
	return d
}

// scroll_with keeps the scroll position in st, which the app owns: it reads where the view
// is and moves it (set st.y, then invalidate the window's prepaint).
pub fn (self_ &Div) scroll_with(st &ScrollState) &Div {
	mut d := unsafe { self_ }
	d.scroll_at = unsafe { st }
	return d
}

// wheel_x makes the plain wheel scroll sideways, for strips that only scroll horizontally.
pub fn (self_ &Div) wheel_x() &Div {
	mut d := unsafe { self_ }
	d.wheel_x = true
	return d
}

pub fn (self_ &Div) overflow_scroll() &Div {
	mut d := unsafe { self_ }
	d.style.overflow_x, d.style.overflow_y = .scroll, .scroll
	return d
}

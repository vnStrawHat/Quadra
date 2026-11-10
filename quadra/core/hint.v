// Hover hints (specs/06): a hitbox's `hint` is a tooltip for a part of an element, such as a
// link in text, which the kit draws once the pointer has rested on it for hint_delay_ms. A
// press or a wheel closes it until the pointer reaches another hitbox or leaves.
module core

import quadra.prim

// hint_delay_ms is how long the pointer rests before a hint (and a kit tooltip) shows.
pub const hint_delay_ms = 500.0

// The hints of links that open on ctrl-click: web and mail links open, anchors and files
// are followed (specs/08, 18, 21, 22).
pub const hint_ctrl_open = 'Ctrl+click to open'
pub const hint_ctrl_follow = 'Ctrl+click to follow'

// link_hint is the hint over a ctrl-click link: web and mail links open, the rest (anchors,
// files) are followed. A prefix test: nothing is allocated per link and prepaint.
pub fn link_hint(url string) string {
	web := url.starts_with('https:') || url.starts_with('http:') || url.starts_with('mailto:')
	return if web { hint_ctrl_open } else { hint_ctrl_follow }
}

// HoverHint is the hint of the hovered path.
pub struct HoverHint {
pub:
	id     u64 // the hitbox, 0 when none
	text   string
	bounds prim.Rect
	since  f64 // when the pointer reached it
	closed bool
pub mut:
	shown bool // drawn by the last prepaint
}

// note_hint takes the deepest hitbox of path with a hint; one that moved is a new hint (the
// same slot over another link). A new hint arms a prepaint for when it is due; true when a
// drawn hint changed and needs a prepaint at once.
fn (mut w Window) note_hint(path []int) bool {
	mut idx := -1
	for i in path {
		if w.hitboxes[i].hint != '' {
			idx = i
			break
		}
	}
	shown := w.hint.shown
	if idx < 0 {
		if w.hint.id != 0 {
			w.hint = HoverHint{}
		}
		return shown
	}
	hb := w.hitboxes[idx]
	if hb.id == w.hint.id && hb.hint == w.hint.text && hb.bounds == w.hint.bounds {
		return false
	}
	app := w.app
	w.hint = HoverHint{
		id:     hb.id
		text:   hb.hint
		bounds: hb.bounds
		since:  app.plat.now_ms()
	}
	w.prepaint_after(int(hint_delay_ms) + 1)
	return shown
}

// close_hint hides the hint until the pointer reaches another hitbox (a press, a wheel).
fn (mut w Window) close_hint() {
	if w.hint.id == 0 || w.hint.closed {
		return
	}
	w.hint = HoverHint{
		...w.hint
		closed: true
	}
	if w.hint.shown {
		w.invalidate_prepaint()
	}
}

// prepaint_after prepaints the window again after ms, without a closure (hover delays).
// Requests coalesce: a caller woken early asks again for the time left.
fn (mut w Window) prepaint_after(ms int) {
	if w.prepaint_timer != 0 {
		return
	}
	mut app := w.app
	w.prepaint_timer = app.plat.set_timer(ms, false)
	app.timers[w.prepaint_timer] = TimerEntry{
		reprepaint: w.id
	}
}

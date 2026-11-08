// Mouse dispatch: hit-testing, capture/bubble, hover, press and click synthesis.
module core

import quadra.prim
import quadra.platform

const double_click_ms = 500.0
const double_click_px = f32(4)

// hit_path returns hitbox indices from the topmost one under p down to the root.
pub fn (w &Window) hit_path(p prim.Point) []int {
	for i := w.hitboxes.len - 1; i >= 0; i-- {
		if w.hitboxes[i].bounds.contains(p) {
			mut path := []int{}
			mut j := i
			for j >= 0 {
				path << j
				j = w.hitboxes[j].parent
			}
			return path
		}
	}
	return []int{}
}

fn (mut w Window) update_hover() {
	path := if w.mouse_in { w.hit_path(w.mouse) } else { []int{} }
	mut next := map[u64]bool{}
	mut cursor := platform.Cursor.arrow
	mut cursor_set := false
	for i in path {
		next[w.hitboxes[i].id] = true
		if !cursor_set && w.hitboxes[i].has_cursor {
			cursor = w.hitboxes[i].cursor
			cursor_set = true
		}
	}
	mut changed := []int{}
	mut reprepaint := false
	for i, hb in w.hitboxes {
		if next[hb.id] != w.hovered[hb.id] {
			if hb.hover_sensitive {
				changed << i
			}
			reprepaint = reprepaint || hb.hover_prepaint
		}
	}
	repaint := changed.len > 0
	w.hovered = next.move()
	w.pw.set_cursor(cursor)
	if reprepaint && !w.dirty && !w.in_prepaint {
		w.invalidate_prepaint()
	} else if repaint && !w.dirty {
		w.restyle(changed)
	}
}

// dispatch runs handlers of `kind` along path: capture (root first), then bubble.
fn (mut w Window) dispatch(kind HandlerKind, path []int, event voidptr, discrete bool) {
	w.stopped = false
	mut app := w.app
	for capture in [true, false] {
		for k := 0; k < path.len; k++ {
			idx := if capture { path[path.len - 1 - k] } else { path[k] }
			hb := w.hitboxes[idx]
			for h in w.handlers[hb.handlers_start..hb.handlers_end] {
				if h.kind == kind && h.capture == capture {
					w.target_bounds = hb.bounds
					w.dispatching = idx
					app.invoke(h.listener, event, w, discrete)
					if w.stopped {
						return
					}
				}
			}
		}
	}
}

fn (mut w Window) on_mouse(e platform.MouseEvent) {
	p := prim.pt(e.x, e.y)
	match e.action {
		.move {
			w.mouse, w.mouse_in = p, true
			w.update_hover()
			w.note_thumb_activity(p, false)
			if w.debug.inspector {
				w.invalidate_paint()
			}
			if w.thumb_move(p) {
				return
			}
			if w.drag_move(p) {
				return
			}
			ev := MouseMoveEvent{e.x, e.y, w.down_path.len > 0, e.mods}
			w.dispatch(.mouse_move, w.capture_path() or { w.hit_path(p) }, &ev, false)
		}
		.leave {
			w.mouse_in = false
			w.update_hover()
		}
		.down {
			w.mouse, w.mouse_in = p, true
			if e.button == .left && w.thumb_down(p) {
				return
			}
			app := w.app
			now := app.plat.now_ms()
			close_to_last := prim.max(prim.max(p.x - w.last_pos.x, w.last_pos.x - p.x), prim.max(p.y - w.last_pos.y,
				w.last_pos.y - p.y)) <= double_click_px
			w.click_count = if now - w.last_click < double_click_ms && close_to_last {
				w.click_count + 1
			} else {
				1
			}
			w.last_click, w.last_pos = now, p
			path := w.hit_path(p)
			w.down_path = path.map(w.hitboxes[it].id)
			w.pressed = map[u64]bool{}
			for id in w.down_path {
				w.pressed[id] = true
			}
			// Clicking focuses the deepest focusable element under the mouse.
			mut focus := u64(0)
			for i in path {
				if w.hitboxes[i].focus_id != 0 {
					focus = w.hitboxes[i].focus_id
					break
				}
			}
			w.set_focus(focus, false)
			if e.button == .left {
				w.arm_drag(path, p)
			}
			ev := MouseDownEvent{e.x, e.y, e.button, w.click_count, e.mods}
			w.dispatch(.mouse_down, path, &ev, false)
			w.invalidate_if_active_styles(path)
		}
		.up {
			if w.thumb_up() {
				return
			}
			if w.drag_release(p) {
				w.capture = 0
				return
			}
			path := w.hit_path(p)
			ev := MouseUpEvent{e.x, e.y, e.button, e.mods}
			w.dispatch(.mouse_up, w.capture_path() or { path }, &ev, false)
			w.capture = 0
			// A click goes to elements under both the press and the release.
			mut both := []int{}
			for i in path {
				if w.hitboxes[i].id in w.down_path {
					both << i
				}
			}
			was_pressed := w.down_path.len > 0
			released := w.down_path.clone()
			w.down_path = []u64{}
			w.pressed = map[u64]bool{}
			if both.len > 0 {
				click := ClickEvent{e.x, e.y, e.button, w.click_count, e.mods}
				w.dispatch(.click, both, &click, true)
			}
			if was_pressed {
				// Restyle what was pressed, wherever the mouse is now.
				mut ids := []int{}
				for i, hb in w.hitboxes {
					if hb.hover_sensitive && hb.id in released {
						ids << i
					}
				}
				if ids.len > 0 {
					w.restyle(ids)
				}
			}
		}
	}
}

fn (mut w Window) invalidate_if_active_styles(path []int) {
	mut ids := []int{}
	for i in path {
		if w.hitboxes[i].hover_sensitive {
			ids << i
		}
	}
	if ids.len > 0 {
		w.restyle(ids)
	}
}

fn (mut w Window) on_wheel(e platform.WheelEvent) {
	line := f32(40) // logical px per wheel line
	ev := ScrollEvent{
		dx:      if e.precise { e.dx } else { e.dx * line }
		dy:      if e.precise { e.dy } else { e.dy * line }
		precise: e.precise
		x:       e.x
		y:       e.y
		mods:    e.mods
	}
	w.note_thumb_activity(prim.pt(e.x, e.y), true)
	w.dispatch(.scroll, w.hit_path(prim.pt(e.x, e.y)), &ev, false)
}

// capture_path is the path of the capturing hitbox while a drag is captured.
fn (w &Window) capture_path() ?[]int {
	if w.capture == 0 {
		return none
	}
	for i, hb in w.hitboxes {
		if hb.id == w.capture {
			return [i]
		}
	}
	return none
}

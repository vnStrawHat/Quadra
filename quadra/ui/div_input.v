// Div builder: focus, keyboard, actions, semantics and scrolling.
module ui

import quadra.core
import quadra.platform
import quadra.prim

pub struct KeySpec {
pub:
	kind        core.KeyHandlerKind
	action_type string
	listener    core.AnyListener
}

pub struct Semantics {
pub mut:
	role     core.Role
	label    string
	value    string
	checked  bool
	disabled bool
	selected bool
	expanded bool
}

pub fn (self_ &Div) track_focus(h core.FocusHandle) &Div {
	mut d := unsafe { self_ }
	d.focus_id = h.id
	return d
}

pub fn (self_ &Div) tab_index(n int) &Div {
	mut d := unsafe { self_ }
	d.tab_idx = n
	return d
}

pub fn (self_ &Div) key_context(ctx string) &Div {
	mut d := unsafe { self_ }
	d.key_ctx = ctx
	return d
}

pub fn (self_ &Div) focus_ring(p Patch) &Div {
	mut d := unsafe { self_ }
	d.states_mut().focus = p
	return d
}

pub fn (self_ &Div) on_key_down(l core.Listener[core.KeyDownEvent]) &Div {
	mut d := unsafe { self_ }
	d.keys << KeySpec{.key_down, '', l.any}
	return d
}

pub fn (self_ &Div) on_key_up(l core.Listener[core.KeyUpEvent]) &Div {
	mut d := unsafe { self_ }
	d.keys << KeySpec{.key_up, '', l.any}
	return d
}

pub fn (self_ &Div) on_text_input(l core.Listener[core.TextInputEvent]) &Div {
	mut d := unsafe { self_ }
	d.keys << KeySpec{.text, '', l.any}
	return d
}

pub fn (self_ &Div) on_ime(l core.Listener[core.ImeInputEvent]) &Div {
	mut d := unsafe { self_ }
	d.keys << KeySpec{.ime, '', l.any}
	return d
}

pub fn (self_ &Div) on_focus_change(l core.Listener[core.FocusChangeEvent]) &Div {
	mut d := unsafe { self_ }
	d.keys << KeySpec{.focus_change, '', l.any}
	return d
}

// on_action handles a bound action (see App.bind) while focus is inside this Div.
pub fn (self_ &Div) on_action[A](l core.Listener[A]) &Div {
	mut d := unsafe { self_ }
	d.keys << KeySpec{.action, typeof[A]().name, l.any}
	return d
}

pub fn (self_ &Div) role(r core.Role) &Div {
	mut d := unsafe { self_ }
	d.sem.role = r
	return d
}

pub fn (self_ &Div) label(s string) &Div {
	mut d := unsafe { self_ }
	d.sem.label = s
	return d
}

pub fn (self_ &Div) sem_value(s string) &Div {
	mut d := unsafe { self_ }
	d.sem.value = s
	return d
}

pub fn (self_ &Div) checked(on bool) &Div {
	mut d := unsafe { self_ }
	d.sem.checked = on
	return d
}

pub fn (self_ &Div) selected(on bool) &Div {
	mut d := unsafe { self_ }
	d.sem.selected = on
	return d
}

pub fn (self_ &Div) expanded(on bool) &Div {
	mut d := unsafe { self_ }
	d.sem.expanded = on
	return d
}

pub fn (self_ &Div) disabled(on bool) &Div {
	mut d := unsafe { self_ }
	d.sem.disabled = on
	return d
}

// prepaint_input registers key nodes, semantics and scrolling; returns the key node and
// the semantic node it pushed, or -1.
fn (mut d Div) prepaint_input(bounds prim.Rect, mut cx core.PrepaintCx) (int, int) {
	mut node := -1
	mut sem := -1
	if d.focus_id != 0 || d.key_ctx != '' || d.keys.len > 0 {
		node = cx.insert_key_node(d.focus_id, d.key_ctx,
			if d.sem.disabled { -1 } else { d.tab_idx }, bounds)
		for k in d.keys {
			cx.on_key(node, k.kind, k.action_type, k.listener)
		}
		cx.push_key_node(node)
	}
	if d.sem.role != .none || d.sem.label != '' {
		sem = cx.register_semantics(core.SemanticNode{
			role:     d.sem.role
			label:    d.sem.label
			value:    d.sem.value
			checked:  d.sem.checked
			disabled: d.sem.disabled
			selected: d.sem.selected
			expanded: d.sem.expanded
			focused:  d.focus_id != 0 && cx.win.focused == d.focus_id
			test_id:  d.test_id
			bounds:   bounds
			focus_id: d.focus_id
		})
		cx.push_semantics(sem)
	}
	d.scroll = prim.Point{}
	if d.style.overflow_x == .scroll || d.style.overflow_y == .scroll {
		mut st := if d.scroll_at != unsafe { nil } { d.scroll_at } else { cx.state(ScrollState{}) }
		content := cx.tree.content_size(d.node)
		max_x := if d.style.overflow_x == .scroll {
			prim.max(0, content.width - bounds.width)
		} else {
			0
		}
		max_y := if d.style.overflow_y == .scroll {
			prim.max(0, content.height - bounds.height)
		} else {
			0
		}
		st.x = prim.clamp(st.x, 0, max_x)
		st.y = prim.clamp(st.y, 0, max_y)
		d.scroll = prim.pt(st.x, st.y)
		d.scroll_target = ScrollTarget{st, max_x, max_y, cx.win, d.wheel_x}
		if d.hitbox < 0 {
			d.hitbox = cx.insert_hitbox(bounds, core.HitboxOptions{})
			cx.push_hitbox(d.hitbox)
			d.own_scroll_hitbox = true
		}
		cx.on_mouse(d.hitbox, .scroll, core.AnyListener{
			holder: voidptr(&d.scroll_target)
			call:   scroll_handler
		}, false)
	}
	return node, sem
}

// scroll_offset reports the current scroll position (for scrollbars).
pub fn (d &Div) scroll_offset() prim.Point {
	return d.scroll
}

// layout_node exposes the layout node of the Div (scroll areas read its content size).
pub fn (d &Div) layout_node() int {
	return int(d.node)
}

// window_region marks this element as part of a custom titlebar: .caption drags the window
// (double click maximizes), .minimize / .maximize / .close act as the window buttons and
// give the OS its native behaviour (snap layouts over maximize on Windows 11).
pub fn (self_ &Div) window_region(r platform.HitRegion) &Div {
	mut d := unsafe { self_ }
	d.region = r
	return d
}

// --- drag and drop (core/drag.v) ---

// draggable makes this element a drag source carrying kind and value; preview builds what
// follows the pointer (nil: nothing follows).
pub fn (self_ &Div) draggable(kind string, value string, preview core.PreviewFn) &Div {
	mut d := unsafe { self_ }
	d.states_mut().drag = core.DragSource{
		payload: core.DragPayload{kind, value}
		preview: preview
	}
	return d
}

// drag_source makes this element a drag source with every option (core.DragSource).
pub fn (self_ &Div) drag_source(src core.DragSource) &Div {
	mut d := unsafe { self_ }
	d.states_mut().drag = src
	return d
}

// on_drop makes this element a drop target for drags of kind.
pub fn (self_ &Div) on_drop(kind string, l core.Listener[core.DropEvent]) &Div {
	mut d := unsafe { self_ }
	d.states_mut().drop_kind = kind
	d.handlers << Handler{.drop, false, l.any}
	return d
}

// drag_over restyles this drop target while an accepted drag is over it.
pub fn (self_ &Div) drag_over(p Patch) &Div {
	mut d := unsafe { self_ }
	d.states_mut().drop = p
	return d
}

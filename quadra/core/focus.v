// Focus handles and the key-dispatch tree built during prepaint.
module core

import quadra.prim

pub struct FocusHandle {
pub:
	id u64
}

pub fn (mut cx Cx) focus_handle() FocusHandle {
	cx.app.next_focus++
	return FocusHandle{cx.app.next_focus}
}

pub fn (h FocusHandle) is_focused(cx &Cx) bool {
	return cx.win != unsafe { nil } && cx.win.focused == h.id
}

// is_focused_in reports focus in any window of the app (focus ids are unique app-wide).
// Use it in observers and tasks, whose context has no window or another one.
pub fn (h FocusHandle) is_focused_in(app &App) bool {
	return app.windows.any(it.focused == h.id)
}

// focus moves keyboard focus to this handle in the current window.
pub fn (h FocusHandle) focus(mut cx Cx) {
	if cx.win != unsafe { nil } {
		cx.win.set_focus(h.id, false)
	}
}

pub fn (mut cx Cx) blur() {
	if cx.win != unsafe { nil } {
		cx.win.set_focus(0, false)
	}
}

// KeyNode is one element on the key-dispatch tree (focus targets, key contexts, key handlers).
pub struct KeyNode {
pub:
	parent    int
	focus_id  u64
	context   string
	tab_index int
	bounds    prim.Rect
pub mut:
	handlers_start int
	handlers_end   int
}

pub enum KeyHandlerKind {
	key_down
	key_up
	text
	ime
	action
	focus_change
}

pub struct KeyHandler {
pub:
	kind        KeyHandlerKind
	action_type string
	listener    AnyListener
}

// focus_path returns key-node indices from the focused node up to the root.
fn (w &Window) focus_path() []int {
	if w.focused == 0 {
		return []int{}
	}
	for i, n in w.key_nodes {
		if n.focus_id == w.focused {
			mut path := []int{}
			mut j := i
			for j >= 0 {
				path << j
				j = w.key_nodes[j].parent
			}
			return path
		}
	}
	return []int{}
}

// focus_order lists focusable node indices in tab order (tab_index first, then paint order).
fn (w &Window) focus_order() []int {
	mut items := []FocusItem{}
	for i, n in w.key_nodes {
		if n.focus_id != 0 && n.tab_index >= 0 {
			items << FocusItem{n.tab_index, i}
		}
	}
	items.sort_with_compare(compare_focus) // no closure: V never frees them
	return items.map(it.index)
}

struct FocusItem {
	tab   int
	index int
}

fn compare_focus(a &FocusItem, b &FocusItem) int {
	ta, tb := a.tab, b.tab
	if ta != tb && ta > 0 && tb > 0 {
		return ta - tb
	}
	if ta > 0 && tb == 0 {
		return -1
	}
	if tb > 0 && ta == 0 {
		return 1
	}
	return a.index - b.index
}

fn (mut w Window) move_focus(step int) {
	order := w.focus_order()
	if order.len == 0 {
		return
	}
	mut cur := -1
	for k, i in order {
		if w.key_nodes[i].focus_id == w.focused {
			cur = k
		}
	}
	next := if cur < 0 {
		if step > 0 { 0 } else { order.len - 1 }
	} else {
		(cur + step + order.len) % order.len
	}
	w.set_focus(w.key_nodes[order[next]].focus_id, true)
}

// set_focus changes focus; keyboard means the focus ring should show (focus-visible).
pub fn (mut w Window) set_focus(id u64, keyboard bool) {
	if w.focused == id {
		w.focus_visible = keyboard
		return
	}
	old := w.focused
	w.focused = id
	w.focus_visible = keyboard
	w.notify_focus_change(old, false)
	w.notify_focus_change(id, true)
	w.invalidate_prepaint() // semantics record focus
}

fn (mut w Window) notify_focus_change(id u64, focused bool) {
	if id == 0 {
		return
	}
	for n in w.key_nodes {
		if n.focus_id == id {
			ev := FocusChangeEvent{focused}
			for h in w.key_handlers[n.handlers_start..n.handlers_end] {
				if h.kind == .focus_change {
					w.app.invoke(h.listener, &ev, w, true)
				}
			}
		}
	}
}

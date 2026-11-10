// Keyboard, text and IME routing along the focus path.
module core

import quadra.prim
import quadra.platform

fn (mut w Window) on_input(e platform.Event) {
	match e {
		platform.KeyEvent {
			w.set_mods(e.mods)
			if e.down {
				w.on_key_down(e)
			} else {
				ev := KeyUpEvent{e.key, e.mods}
				w.dispatch_keys(.key_up, '', prim.hold(&ev), false)
			}
		}
		platform.TextEvent {
			ev := TextInputEvent{e.text}
			w.dispatch_first(.text, prim.hold(&ev))
		}
		platform.ImeEvent {
			ev := ImeInputEvent{e.kind, e.text, e.cursor}
			w.dispatch_first(.ime, prim.hold(&ev))
		}
		else {}
	}
}

fn (mut w Window) on_key_down(e platform.KeyEvent) {
	if e.key == .escape && w.cancel_drag() {
		return
	}
	path := w.focus_path()
	mut contexts := [][]string{}
	for k := path.len - 1; k >= 0; k-- {
		ctx := w.key_nodes[path[k]].context
		if ctx != '' {
			contexts << ctx.fields()
		}
	}
	seq := w.pending_keys.clone()
	mut full := seq.clone()
	full << Keystroke{e.key, e.mods}
	res, b := w.app.keymap.resolve(full, contexts)
	match res {
		.matched {
			w.pending_keys.clear()
			if w.dispatch_action(b.action_type, b.action) {
				return
			}
		}
		.pending {
			w.pending_keys = full
			return
		}
		.none {
			w.pending_keys.clear()
		}
	}

	// The devtools keys (ctrl-shift-p, -f, -i) come after the app's bindings: an app that
	// binds one (a command palette on ctrl-shift-p) keeps it.
	if w.debug_key(e) {
		return
	}
	ev := KeyDownEvent{e.key, e.mods, e.repeat}
	w.stopped = false
	w.dispatch_keys(.key_down, '', prim.hold(&ev), true)
	if !w.stopped && e.key == .tab && !e.mods.has(.ctrl) && !e.mods.has(.alt) {
		w.move_focus(if e.mods.has(.shift) { -1 } else { 1 })
	}
}

// dispatch_action runs the deepest action handler of that type on the focus path,
// then app-level handlers. Returns whether anything handled it.
pub fn (mut w Window) dispatch_action(action_type string, action prim.Holder) bool {
	mut path := w.focus_path()
	if path.len == 0 {
		// Nothing focused: offer the action to every handler, topmost (overlays) first.
		// Actions are typed, so only elements that declared this action can take it.
		path = []int{len: w.key_nodes.len, init: w.key_nodes.len - 1 - index}
	}
	mut app := w.app
	for i in path {
		n := w.key_nodes[i]
		for h in w.key_handlers[n.handlers_start..n.handlers_end] {
			if h.kind == .action && h.action_type == action_type {
				w.target_bounds = n.bounds
				w.skipped = false
				app.invoke_held(h.listener, action, w, true)
				if w.skipped {
					w.skipped = false
					continue // the handler declined: try outer handlers
				}
				return true
			}
		}
	}
	for h in app.global_actions {
		if h.action_type == action_type {
			w.skipped = false
			app.invoke_held(h.listener, action, w, true)
			if w.skipped { // nothing to do: the key goes on to the focused element
				w.skipped = false
				continue
			}
			return true
		}
	}
	return false
}

// dispatch_keys runs handlers of `kind` from the focused node up to the root (bubble).
fn (mut w Window) dispatch_keys(kind KeyHandlerKind, action_type string, event prim.Holder, discrete bool) {
	w.stopped = false
	path := w.focus_path()
	mut app := w.app
	for i in path {
		n := w.key_nodes[i]
		for h in w.key_handlers[n.handlers_start..n.handlers_end] {
			if h.kind == kind && h.action_type == action_type {
				w.target_bounds = n.bounds
				app.invoke_held(h.listener, event, w, discrete)
				if w.stopped {
					return
				}
			}
		}
	}
}

// dispatch_first delivers text and IME input to the deepest handler only.
fn (mut w Window) dispatch_first(kind KeyHandlerKind, event prim.Holder) {
	path := w.focus_path()
	mut app := w.app
	for i in path {
		n := w.key_nodes[i]
		for h in w.key_handlers[n.handlers_start..n.handlers_end] {
			if h.kind == kind {
				w.target_bounds = n.bounds
				app.invoke_held(h.listener, event, w, true)
				return
			}
		}
	}
}

// on_action registers an app-wide handler for an action type (used when no focused
// element handles it), e.g. quit or open settings.
pub fn (mut app App) on_action[A](l Listener[A]) {
	app.escaped_in_render()
	app.global_actions << KeyHandler{
		kind:        .action
		action_type: typeof[A]().name
		listener:    l.any
	}
}

// dispatch sends an action as if its key binding was pressed in window `win`.
pub fn (mut cx Cx) dispatch[A](action A) {
	if cx.win != unsafe { nil } {
		cx.win.dispatch_action(typeof[A]().name, heap_hold(action))
	}
}

// skip declines the action being handled, so outer handlers (and, failing those, plain
// key handlers) get it. A single-line input skips up/down for its owner, for example.
pub fn (mut cx Cx) skip() {
	if cx.win != unsafe { nil } {
		cx.win.skipped = true
	}
}

// The overlay host: kit.install wraps every window root in a Host view that renders
// modals, popups and toasts above the app, so apps never install a root wrapper.
module kit

import quadra.core
import quadra.prim
import quadra.ui

pub struct Dismiss {}

pub enum ToastKind {
	info
	success
	warning
	error
}

pub struct Toast {
pub:
	id   int
	win  u32
	kind ToastKind
	text string
}

pub struct Modal {
pub:
	id          int
	win         u32
	side        ModalSide
	title       string
	body        core.BoundRef[Theme, core.Element]
	footer      core.BoundRef[Theme, core.Element]
	width       f32 = 420
	description string
	alert       bool
}

pub struct Popup {
pub:
	id        int
	win       u32
	anchor    prim.Rect
	placement ui.Placement
	content   core.BoundRef[Theme, core.Element]
	focus     u64 // focused when the popup opened; it gets the focus back on close
}

// Overlays is the app-wide overlay state (a global model).
pub struct Overlays {
pub mut:
	modals  []Modal
	popups  []Popup
	toasts  []Toast
	notices []Notice
	next_id int
}

pub struct Host {
pub:
	root core.AnyView
}

// install sets the default theme, the overlay host and default bindings. Call it once,
// before opening windows.
pub fn install(mut app core.App) ! {
	mut cx := core.Cx{
		app: app
	}
	// Without the bundled faces (an old DirectWrite) text falls back to the system face.
	load_faces(mut app) or {}
	if cx.peek_global[Theme]() == none {
		set_theme(mut app, light())
	}
	cx.set_global(Overlays{})
	cx.set_global(SettingsUi{})
	app.wrap_root = fn (mut cx core.Cx, root core.AnyView) core.AnyView {
		return cx.new_view(Host{root}).any()
	}
	app.bind('escape', Dismiss{})!
	app.on_action(core.handler(fn (_ &Dismiss, mut cx core.Cx) {
		if !dismiss_top(mut cx) {
			cx.skip() // no overlay: Esc belongs to the focused element (a terminal)
		}
	}))
}

@[markused]
pub fn (mut h Host) render(mut cx core.ViewCx[Host]) core.Element {
	ov := cx.global[Overlays]() or { &Overlays{} }
	t := theme(cx)
	win := cx.win.id
	cx.win.clear_color = t.colors.background // windows opened after set_theme
	cx.win.focus_color = t.p.border_focused
	cx.win.scroll_color = t.p.scroll_thumb
	cx.win.thumb_style = thumb_style(t)
	cx.win.selection_color = if t.p.selection.a > 0 {
		t.p.selection
	} else {
		t.code.editor.selection
	}
	mut root :=
		ui.div().size_full().bg(t.colors.background).text_color(t.colors.foreground).child(cx.child_any(h.root)).child(HintLayer{})
	for p in ov.popups.filter(it.win == win) {
		// A transparent full-window layer closes the popup on outside clicks.
		root.child(ui.anchored(ui.div().size_full().on_mouse_down(core.handler_with(p.id,
			close_popup_on[core.MouseDownEvent]))).fill_window().layer(10 + p.id * 2))
		root.child(ui.anchored(p.content.run(t)).at(p.anchor).placement(p.placement).layer(11 +
			p.id * 2))
	}
	for m in ov.modals.filter(it.win == win) {
		root.child(ui.anchored(modal_frame(m, t, cx.win.size())).fill_window().layer(1000 + m.id))
	}
	toasts := newest(ov.toasts.filter(it.win == win))
	notices := newest(ov.notices.filter(it.win == win))
	if t.ctl.tinted_notices {
		if toasts.len + notices.len > 0 {
			root.child(tinted_stack(notices, toasts, t, cx.win.size()))
		}
		return root
	}
	if toasts.len > 0 {
		// Bottom center, just above the status bar (specs/15).
		mut stack := ui.div().flex_col().gap(8).items_center()
		for toast in toasts {
			stack.child(toast_card(toast, t))
		}
		sz := cx.win.size()
		root.child(ui.anchored(stack).at(prim.rect(sz.width / 2, sz.height - t.statusbar_h - 8, 0,
			0)).placement(.top).layer(5000))
	}
	if notices.len > 0 {
		root.child(notice_stack(notices, t, cx.win.size()))
	}
	return root
}

// dismiss_top closes the most recent popup, else the most recent modal, in this window.
pub fn dismiss_top(mut cx core.Cx) bool {
	if cx.win == unsafe { nil } {
		return false
	}
	win := cx.win.id
	mut ov := cx.peek_global[Overlays]() or { return false }
	for i := ov.popups.len - 1; i >= 0; i-- {
		if ov.popups[i].win == win {
			restore_focus(mut cx, ov.popups[i])
			ov.popups.delete(i)
			ov.free_idle(mut cx)
			cx.touch_global[Overlays]()
			return true
		}
	}
	for i := ov.modals.len - 1; i >= 0; i-- {
		if ov.modals[i].win == win {
			ov.modals.delete(i)
			ov.free_idle(mut cx)
			cx.touch_global[Overlays]()
			return true
		}
	}
	return false
}

const max_shown_notices = 10

// newest is what a stack shows: the last ten, the rest wait for room.
fn newest[T](all []T) []T {
	return if all.len > max_shown_notices { all[all.len - max_shown_notices..] } else { all }
}

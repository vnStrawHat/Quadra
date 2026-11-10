// Window events from the platform, and lifecycle hooks: the close flow ("save changes?"), dropped files, forwarded
// command lines of a single-instance app.
module core

import quadra.prim
import quadra.platform

// CloseRequest is passed to close handlers; call cx.prevent_close() to keep the window.
pub struct CloseRequest {}

// on_close_requested registers the view's close handler for the current window (once per
// view, so calling it on every render is fine). Handlers run before the window closes.
pub fn (mut cx Cx) on_close_requested(l Listener[CloseRequest]) {
	cx.win.close_handlers[l.any.view] = l.any
}

// prevent_close keeps the window open; call it from a close handler, then close later with
// cx.close_window() (for example after a confirm dialog).
pub fn (mut cx Cx) prevent_close() {
	cx.win.close_vetoed = true
}

// request_close asks to close the current window, as the OS close button does: the close
// handlers run first and may keep it open.
pub fn (mut cx Cx) request_close() {
	if cx.win != unsafe { nil } {
		cx.win.request_close()
	}
}

// close_window closes the current window without asking the close handlers again.
pub fn (mut cx Cx) close_window() {
	if cx.win != unsafe { nil } {
		cx.win.pw.close()
	}
}

// on_files_dropped registers the view's handler for files dropped anywhere on the window.
pub fn (mut cx Cx) on_files_dropped(l Listener[platform.FileDropEvent]) {
	cx.win.drop_handlers[l.any.view] = l.any
}

// on_instance_args registers the handler for command lines forwarded by later launches.
pub fn (mut cx Cx) on_instance_args(l Listener[platform.InstanceArgs]) {
	cx.app.instance_handlers[l.any.view] = l.any
}

fn (mut w Window) request_close() {
	w.close_vetoed = false
	ev := CloseRequest{}
	mut app := w.app
	for _, l in w.close_handlers {
		app.invoke(l, &ev, w, false)
	}
	if !w.close_vetoed {
		w.pw.close()
	}
}

fn (mut w Window) deliver_drop(e platform.FileDropEvent) {
	mut app := w.app
	for _, l in w.drop_handlers {
		app.invoke(l, &e, w, true)
	}
}

// deliver_instance_args runs the forwarded-arguments handlers (any window may receive it).
fn (mut app App) deliver_instance_args(e platform.InstanceArgs, w &Window) {
	for _, l in app.instance_handlers {
		app.invoke(l, &e, w, true)
	}
}

// handle routes one platform event of this window.
fn (mut w Window) handle(e platform.Event) {
	match e {
		platform.ResizeEvent, platform.ScaleEvent {
			dw, dh := w.device_size()
			w.surface.resize(dw, dh) or { eprintln('quadra: resize failed: ${err}') }
			w.invalidate()
		}
		platform.MouseEvent {
			w.on_mouse(e)
		}
		platform.WheelEvent {
			w.on_wheel(e)
		}
		platform.CloseRequested {
			w.request_close()
		}
		platform.A11yEvent {
			w.on_a11y(e)
		}
		platform.FileDropEvent {
			w.deliver_drop(e)
		}
		platform.InstanceArgs {
			mut app := w.app
			app.deliver_instance_args(e, w)
		}
		platform.TrayEvent {
			mut app := w.app
			for _, l in app.tray_handlers {
				app.invoke(l, &e, w, true)
			}
		}
		platform.WindowClosed {
			w.closed = true
			w.app.window_closed(w)
		}
		platform.AppearanceChanged {
			// Only the first window runs the hooks; they update app-wide state.
			mut app := w.app
			if app.windows.len > 0 && app.windows[0].id == w.id {
				for hook in app.appearance_hooks {
					hook(mut app)
				}
			}
		}
		platform.FocusEvent {
			if !e.focused {
				w.pressed = map[u64]bool{}
				w.down_path = []u64{}
			}
		}
		else {
			w.on_input(e)
		}
	}
}

// NoSurface stands in for the GPU surface until the first frame attaches the real one.
struct NoSurface {}

fn (s NoSurface) waitable() voidptr {
	return unsafe { nil }
}

fn (s NoSurface) read_rgba() ![]u8 {
	return error('quadra: the window has not drawn a frame yet')
}

fn (s NoSurface) width() int {
	return 0
}

fn (s NoSurface) height() int {
	return 0
}

fn (mut s NoSurface) resize(_ int, _ int) ! {}

fn (mut s NoSurface) present() ! {}

fn (w &Window) device_size() (int, int) {
	s := w.pw.size()
	k := w.pw.scale()
	return int(s.width * k + 0.5), int(s.height * k + 0.5)
}

// attach_surface creates the window's GPU surface at its current size.
fn (mut w Window) attach_surface() ! {
	dw, dh := w.device_size()
	mut app := w.app
	w.surface = app.device.new_window_surface(w.pw.native_handle(), dw, dh)!
	w.pw.set_waitable(w.surface.waitable())
	w.surface_ok = true
}

// watch_upgrade polls a device that started on a stand-in (WARP-first start-up) and draws
// every window again once it moved to the real one.
fn (mut app App) watch_upgrade() {
	app.upgrade_watch = true
	id := app.plat.set_timer(25, true)
	app.timers[id] = TimerEntry{
		repeat: true
		holder: prim.hold(&UpgradeWatch{id})
		call:   tick_upgrade
	}
}

struct UpgradeWatch {
	id platform.TimerId
}

fn tick_upgrade(holder prim.Holder, _ prim.Holder, mut cx Cx) {
	id := prim.held[UpgradeWatch](holder).id
	mut app := cx.app
	if app.device.try_upgrade() {
		for mut w in app.windows {
			w.invalidate_paint() // the draw attaches the swap chain (after_frame)
		}
	}
	if !app.device.upgrading() {
		app.plat.cancel_timer(id)
		app.timers.delete(id)
	}
}

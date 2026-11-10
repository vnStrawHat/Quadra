// Timers owned by models: cancelled automatically when the owner is released.
module core

import quadra.prim
import quadra.platform

struct TimerEntry {
	owner  u32
	holder prim.Holder
	call   fn (holder prim.Holder, owner prim.Holder, mut cx Cx) = unsafe { nil }
	repeat bool
	cb     ?fn (mut app App) // owner-less timers (framework internals)
	// repaint: a paint-only animation tick for this window (no closure per frame).
	repaint platform.WindowId
	// reprepaint: prepaint this window again (hover delays such as tooltips).
	reprepaint platform.WindowId
}

// after runs cb once after ms milliseconds (no owning model).
pub fn (mut app App) after(ms int, cb fn (mut app App)) platform.TimerId {
	app.escaped_in_render()
	id := app.plat.set_timer(ms, false)
	app.timers[id] = TimerEntry{
		cb: cb
	}
	return id
}

struct TimerHolder[T] {
	f fn (mut v T, mut cx ViewCx[T]) = unsafe { nil }
}

fn call_timer[T](holder prim.Holder, owner prim.Holder, mut cx Cx) {
	h := prim.held[TimerHolder[T]](holder)
	mut v := unsafe { prim.held[T](owner) } // unsafe only quiets a mutability notice
	mut vcx := ViewCx[T]{
		Cx: cx
	}
	h.f(mut v, mut vcx)
}

fn (mut cx ViewCx[T]) add_timer(ms int, repeat bool, f fn (mut v T, mut cx ViewCx[T])) platform.TimerId {
	cx.app.escaped_in_render()
	id := cx.app.plat.set_timer(ms, repeat)
	cx.app.timers[id] = TimerEntry{
		owner:  cx.entity
		holder: prim.hold(&TimerHolder[T]{f})
		call:   call_timer[T]
		repeat: repeat
	}
	return id
}

// set_timeout calls f once after ms milliseconds.
pub fn (mut cx ViewCx[T]) set_timeout(ms int, f fn (mut v T, mut cx ViewCx[T])) platform.TimerId {
	return cx.add_timer(ms, false, f)
}

// set_interval calls f every ms milliseconds until cancelled or the owner is released.
pub fn (mut cx ViewCx[T]) set_interval(ms int, f fn (mut v T, mut cx ViewCx[T])) platform.TimerId {
	return cx.add_timer(ms, true, f)
}

pub fn (mut cx Cx) cancel_timer(id platform.TimerId) {
	cx.app.plat.cancel_timer(id)
	cx.app.timers.delete(id)
}

fn (mut app App) on_timer_fired(id platform.TimerId) {
	entry := app.timers[id] or { return }
	if !entry.repeat {
		app.timers.delete(id)
	}
	if cb := entry.cb {
		app.probe_begin()
		cb(mut app)
		app.probe_end('timer', 0)
		app.flush()
		return
	}
	if entry.reprepaint != 0 {
		if mut win := app.window(entry.reprepaint) {
			win.prepaint_timer = 0
			win.invalidate_prepaint()
		}
		app.flush()
		return
	}
	if entry.repaint != 0 {
		if mut win := app.window(entry.repaint) {
			win.paint_timer = 0
			win.invalidate_paint()
		}
		app.flush()
		return
	}
	if ptr, mut cx := app.enter_owned(entry.owner) {
		app.probe_begin()
		entry.call(entry.holder, ptr, mut cx)
		app.probe_end('timer', entry.owner)
		app.depth--
	}
	// A timer is a discrete event for its owner, like a click.
	if entry.owner != 0 && app.store.slots[entry.owner].alive {
		app.changed(entry.owner)
	}
	app.flush()
}

fn (mut app App) cancel_owned_timers(owner u32) {
	mut gone := []platform.TimerId{}
	for id, e in app.timers {
		if e.owner == owner {
			gone << id
		}
	}
	for id in gone {
		app.plat.cancel_timer(id)
		app.timers.delete(id)
	}
}

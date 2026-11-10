// Listeners: typed event handlers erased for storage in dispatch tables.
// The framework passes the live view at dispatch time, so no closure holds a view.
module core

import quadra.prim

pub struct AnyListener {
pub:
	view   u32
	holder prim.Holder
	call   fn (holder prim.Holder, view prim.Holder, event prim.Holder, mut cx Cx) = unsafe { nil }
}

pub struct Listener[E] {
pub:
	any AnyListener
}

struct ListenerHolder[T, E] {
	f fn (mut v T, e &E, mut cx ViewCx[T]) = unsafe { nil }
}

fn call_listener[T, E](holder prim.Holder, view prim.Holder, event prim.Holder, mut cx Cx) {
	h := prim.held[ListenerHolder[T, E]](holder)
	mut v := unsafe { prim.held[T](view) } // unsafe only quiets a mutability notice
	mut vcx := ViewCx[T]{
		Cx: cx
	}
	h.f(mut v, prim.held[E](event), mut vcx)
}

// listener wraps a handler that receives the current view.
pub fn (cx &ViewCx[T]) listener[E](f fn (mut v T, e &E, mut cx ViewCx[T])) Listener[E] {
	return listener_of[T, E](cx.entity, f)
}

// listener_of builds a listener for a known view id. Use it in callbacks that run after
// render returns (virtual list rows): capture the id, never the context, because a
// `mut cx` parameter points into a stack frame that is gone by then.
pub fn listener_of[T, E](entity u32, f fn (mut v T, e &E, mut cx ViewCx[T])) Listener[E] {
	if entity == max_u32 {
		// Never runs: a direct call keeps V's unused-code pass from dropping call_listener,
		// which is otherwise reached only through the function pointer below (docs/v-notes.md).
		mut cx := Cx{}
		call_listener[T, E](prim.Holder{}, prim.Holder{}, prim.Holder{}, mut cx)
	}
	return Listener[E]{AnyListener{
		view:   entity
		holder: prim.hold(&ListenerHolder[T, E]{f})
		call:   call_listener[T, E]
	}}
}

struct HandlerHolder[E] {
	f fn (e &E, mut cx Cx) = unsafe { nil }
}

fn call_handler[E](holder prim.Holder, _ prim.Holder, event prim.Holder, mut cx Cx) {
	h := prim.held[HandlerHolder[E]](holder)
	h.f(prim.held[E](event), mut cx)
}

// handler wraps a handler that needs no view.
pub fn handler[E](f fn (e &E, mut cx Cx)) Listener[E] {
	return Listener[E]{AnyListener{
		holder: prim.hold(&HandlerHolder[E]{f})
		call:   call_handler[E]
	}}
}

// --- With data: f captures nothing, the data rides in the holder (specs/14, avoiding
// closures). Holders, views and events are checked holders (prim.held, specs/14): outside
// -prod a listener of the wrong view or event type panics. D must not be a pointer type
// (V 0.5.2 mangles the function pointer wrongly, core/bound.v): wrap a pointer in a struct.

struct DataHandlerHolder[D, E] {
	data D
	f    fn (d D, e &E, mut cx Cx) = unsafe { nil }
}

fn call_data_handler[D, E](holder prim.Holder, _ prim.Holder, event prim.Holder, mut cx Cx) {
	h := prim.held[DataHandlerHolder[D, E]](holder)
	h.f(h.data, prim.held[E](event), mut cx)
}

// handler_with wraps a handler that needs no view but some data, without a closure:
// `core.handler_with(spec, fn (s SliderSpec, e &core.KeyDownEvent, mut cx core.Cx) {...})`.
pub fn handler_with[D, E](data D, f fn (d D, e &E, mut cx Cx)) Listener[E] {
	return Listener[E]{AnyListener{
		holder: prim.hold(&DataHandlerHolder[D, E]{data, f})
		call:   call_data_handler[D, E]
	}}
}

struct DataListenerHolder[T, D, E] {
	data D
	f    fn (mut v T, d D, e &E, mut cx ViewCx[T]) = unsafe { nil }
}

fn call_data_listener[T, D, E](holder prim.Holder, view prim.Holder, event prim.Holder, mut cx Cx) {
	h := prim.held[DataListenerHolder[T, D, E]](holder)
	mut v := unsafe { prim.held[T](view) } // unsafe only quiets a mutability notice
	mut vcx := ViewCx[T]{
		Cx: cx
	}
	h.f(mut v, h.data, prim.held[E](event), mut vcx)
}

// listener_with builds a listener for view `entity` that also receives data (a row index,
// an id), without a closure.
pub fn listener_with[T, D, E](entity u32, data D, f fn (mut v T, d D, e &E, mut cx ViewCx[T])) Listener[E] {
	return Listener[E]{AnyListener{
		view:   entity
		holder: prim.hold(&DataListenerHolder[T, D, E]{data, f})
		call:   call_data_listener[T, D, E]
	}}
}

// invoke runs a listener inside an effect scope. Discrete events (click, key, change)
// mark the listening view changed; continuous ones (move, wheel) leave it to the handler.
// The listener must take an E: outside -prod another event type panics.
pub fn (mut app App) invoke[E](l AnyListener, event &E, win &Window, discrete bool) {
	app.invoke_held(l, prim.hold(event), win, discrete)
}

// invoke_held is invoke for an event already in a holder (the framework's dispatchers).
fn (mut app App) invoke_held(l AnyListener, event prim.Holder, win &Window, discrete bool) {
	if l.call == unsafe { nil } {
		return
	}
	if l.view != 0 && (int(l.view) >= app.store.slots.len || !app.store.slots[l.view].alive) {
		return
	}
	view := app.store.holder(l.view)
	mut cx := Cx{
		app:    app
		win:    unsafe { win }
		entity: l.view
	}
	app.depth++
	app.probe_begin()
	l.call(l.holder, view, event, mut cx)
	app.probe_end('handler', l.view)
	if discrete && l.view != 0 {
		app.changed(l.view)
	}
	app.depth--
	app.flush()
}

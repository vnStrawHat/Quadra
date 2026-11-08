// Listeners: typed event handlers erased for storage in dispatch tables.
// The framework passes the live view at dispatch time, so no closure holds a view.
module core

pub struct AnyListener {
pub:
	view   u32
	holder voidptr
	call   fn (holder voidptr, view voidptr, event voidptr, mut cx Cx) = unsafe { nil }
}

pub struct Listener[E] {
pub:
	any AnyListener
}

struct ListenerHolder[T, E] {
	f fn (mut v T, e &E, mut cx ViewCx[T]) = unsafe { nil }
}

fn call_listener[T, E](holder voidptr, view voidptr, event voidptr, mut cx Cx) {
	h := unsafe { &ListenerHolder[T, E](holder) }
	mut v := unsafe { &T(view) }
	mut vcx := ViewCx[T]{
		Cx: cx
	}
	h.f(mut v, unsafe { &E(event) }, mut vcx)
}

// listener wraps a handler that receives the current view.
pub fn (cx &ViewCx[T]) listener[E](f fn (mut v T, e &E, mut cx ViewCx[T])) Listener[E] {
	return listener_of[T, E](cx.entity, f)
}

// listener_of builds a listener for a known view id. Use it in callbacks that run after
// render returns (virtual list rows): capture the id, never the context, because a
// `mut cx` parameter points into a stack frame that is gone by then.
pub fn listener_of[T, E](entity u32, f fn (mut v T, e &E, mut cx ViewCx[T])) Listener[E] {
	return Listener[E]{AnyListener{
		view:   entity
		holder: voidptr(&ListenerHolder[T, E]{f})
		call:   call_listener[T, E]
	}}
}

struct HandlerHolder[E] {
	f fn (e &E, mut cx Cx) = unsafe { nil }
}

fn call_handler[E](holder voidptr, _ voidptr, event voidptr, mut cx Cx) {
	h := unsafe { &HandlerHolder[E](holder) }
	h.f(unsafe { &E(event) }, mut cx)
}

// handler wraps a handler that needs no view.
pub fn handler[E](f fn (e &E, mut cx Cx)) Listener[E] {
	return Listener[E]{AnyListener{
		holder: voidptr(&HandlerHolder[E]{f})
		call:   call_handler[E]
	}}
}

// --- With data: f captures nothing, the data rides in the holder (specs/14, avoiding
// closures). The one cast back to the holder's type is in the generic call_* function, made
// for the same D and E as the holder, so it always matches. D must not be a pointer type
// (V 0.5.2 mangles the function pointer wrongly, core/bound.v): wrap a pointer in a struct.

struct DataHandlerHolder[D, E] {
	data D
	f    fn (d D, e &E, mut cx Cx) = unsafe { nil }
}

fn call_data_handler[D, E](holder voidptr, _ voidptr, event voidptr, mut cx Cx) {
	h := unsafe { &DataHandlerHolder[D, E](holder) }
	h.f(h.data, unsafe { &E(event) }, mut cx)
}

// handler_with wraps a handler that needs no view but some data, without a closure:
// `core.handler_with(spec, fn (s SliderSpec, e &core.KeyDownEvent, mut cx core.Cx) {...})`.
pub fn handler_with[D, E](data D, f fn (d D, e &E, mut cx Cx)) Listener[E] {
	return Listener[E]{AnyListener{
		holder: voidptr(&DataHandlerHolder[D, E]{data, f})
		call:   call_data_handler[D, E]
	}}
}

struct DataListenerHolder[T, D, E] {
	data D
	f    fn (mut v T, d D, e &E, mut cx ViewCx[T]) = unsafe { nil }
}

fn call_data_listener[T, D, E](holder voidptr, view voidptr, event voidptr, mut cx Cx) {
	h := unsafe { &DataListenerHolder[T, D, E](holder) }
	mut v := unsafe { &T(view) }
	mut vcx := ViewCx[T]{
		Cx: cx
	}
	h.f(mut v, h.data, unsafe { &E(event) }, mut vcx)
}

// listener_with builds a listener for view `entity` that also receives data (a row index,
// an id), without a closure.
pub fn listener_with[T, D, E](entity u32, data D, f fn (mut v T, d D, e &E, mut cx ViewCx[T])) Listener[E] {
	return Listener[E]{AnyListener{
		view:   entity
		holder: voidptr(&DataListenerHolder[T, D, E]{data, f})
		call:   call_data_listener[T, D, E]
	}}
}

// invoke runs a listener inside an effect scope. Discrete events (click, key, change)
// mark the listening view changed; continuous ones (move, wheel) leave it to the handler.
pub fn (mut app App) invoke(l AnyListener, event voidptr, win &Window, discrete bool) {
	if l.call == unsafe { nil } {
		return
	}
	if l.view != 0 && (int(l.view) >= app.store.slots.len || !app.store.slots[l.view].alive) {
		return
	}
	mut view := unsafe { nil }
	if l.view != 0 {
		view = app.store.slots[l.view].ptr
	}
	mut cx := Cx{
		app:    app
		win:    unsafe { win }
		entity: l.view
	}
	app.depth++
	l.call(l.holder, view, event, mut cx)
	if discrete && l.view != 0 {
		app.changed(l.view)
	}
	app.depth--
	app.flush()
}

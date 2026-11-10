// `_with` variants of the callback APIs (specs/14, avoiding closures): f takes the data it
// needs as an argument, so the caller makes no closure. V frees no closure made outside a
// render, layout or prepaint frame: one per timer, debounce, update or observer made in a
// handler stays alive for good with what it captured. D must not be a pointer type (V 0.5.2
// mangles the generic function pointer, bound.v): wrap a pointer in a struct. The ViewCx
// ones are free functions: more generic methods on ViewCx[T] trip V (docs/v-notes.md).
module core

import quadra.platform

struct AfterWith[D] {
	d D
	f fn (d D, mut app App) = unsafe { nil }
}

fn call_after_with[D](holder voidptr, _ voidptr, mut cx Cx) {
	h := unsafe { &AfterWith[D](holder) }
	h.f(h.d, mut cx.app)
}

// after_with is `after` for a callback that needs data.
pub fn (mut app App) after_with[D](ms int, data D, f fn (d D, mut app App)) platform.TimerId {
	app.escaped_in_render()
	id := app.plat.set_timer(ms, false)
	app.timers[id] = TimerEntry{
		holder: voidptr(&AfterWith[D]{data, f})
		call:   call_after_with[D]
	}
	return id
}

struct TimerWith[T, D] {
	d D
	f fn (mut v T, d D, mut cx ViewCx[T]) = unsafe { nil }
}

fn call_timer_with[T, D](holder voidptr, owner voidptr, mut cx Cx) {
	h := unsafe { &TimerWith[T, D](holder) }
	mut v := unsafe { &T(owner) }
	mut vcx := ViewCx[T]{
		Cx: cx
	}
	h.f(mut v, h.d, mut vcx)
}

fn add_timer_with[T, D](mut cx ViewCx[T], ms int, repeat bool, data D, f fn (mut v T, d D, mut cx ViewCx[T])) platform.TimerId {
	cx.app.escaped_in_render()
	id := cx.app.plat.set_timer(ms, repeat)
	cx.app.timers[id] = TimerEntry{
		owner:  cx.entity
		holder: voidptr(&TimerWith[T, D]{data, f})
		call:   call_timer_with[T, D]
		repeat: repeat
	}
	return id
}

// set_timeout_with is set_timeout for a callback that needs data.
pub fn set_timeout_with[T, D](mut cx ViewCx[T], ms int, data D, f fn (mut v T, d D, mut cx ViewCx[T])) platform.TimerId {
	return add_timer_with(mut cx, ms, false, data, f)
}

// set_interval_with is set_interval for a callback that needs data.
pub fn set_interval_with[T, D](mut cx ViewCx[T], ms int, data D, f fn (mut v T, d D, mut cx ViewCx[T])) platform.TimerId {
	return add_timer_with(mut cx, ms, true, data, f)
}

// debounce_with is debounce for a callback that needs data (the last call's data wins).
pub fn debounce_with[T, D](mut cx ViewCx[T], key string, ms int, data D, f fn (mut v T, d D, mut cx ViewCx[T])) {
	k := '${cx.entity}/${key}'
	if old := cx.app.debounced[k] {
		cx.cancel_timer(old)
	}
	cx.app.debounced[k] = add_timer_with(mut cx, ms, false, data, f)
}

struct ObserveWith[T, O, D] {
	f fn (mut v T, m Model[O], d D, mut cx ViewCx[T]) = unsafe { nil }
	m Model[O]
	d D
}

fn call_observer_with[T, O, D](holder voidptr, owner voidptr, mut cx Cx) {
	h := unsafe { &ObserveWith[T, O, D](holder) }
	mut v := unsafe { &T(owner) }
	mut vcx := ViewCx[T]{
		Cx: cx
	}
	h.f(mut v, h.m, h.d, mut vcx)
}

// observe_with is observe for a callback that needs data.
pub fn observe_with[T, O, D](mut cx ViewCx[T], m Model[O], data D, f fn (mut v T, m Model[O], d D, mut cx ViewCx[T])) {
	cx.app.escaped_in_render()
	cx.app.observers << Observer{
		owner:  cx.entity
		target: m.id
		holder: voidptr(&ObserveWith[T, O, D]{f, m, data})
		call:   call_observer_with[T, O, D]
	}
}

struct SubscribeWith[T, S, Q, D] {
	f fn (mut v T, m Model[S], e &Q, d D, mut cx ViewCx[T]) = unsafe { nil }
	m Model[S]
	d D
}

fn call_subscriber_with[T, S, Q, D](holder voidptr, owner voidptr, event voidptr, mut cx Cx) {
	h := unsafe { &SubscribeWith[T, S, Q, D](holder) }
	mut v := unsafe { &T(owner) }
	mut vcx := ViewCx[T]{
		Cx: cx
	}
	h.f(mut v, h.m, unsafe { &Q(event) }, h.d, mut vcx)
}

// subscribe_with is subscribe for a callback that needs data.
pub fn subscribe_with[T, S, Q, D](mut cx ViewCx[T], m Model[S], data D, f fn (mut v T, m Model[S], e &Q, d D, mut cx ViewCx[T])) u64 {
	cx.app.escaped_in_render()
	cx.app.next_sub++
	cx.app.subs << Subscription{
		id:         cx.app.next_sub
		owner:      cx.entity
		target:     m.id
		event_type: typeof[Q]().name
		holder:     voidptr(&SubscribeWith[T, S, Q, D]{f, m, data})
		call:       call_subscriber_with[T, S, Q, D]
	}
	return cx.app.next_sub
}

// update_view_with is update_view for a callback that needs data.
pub fn (mut cx Cx) update_view_with[T, D](v View[T], data D, f fn (mut v T, d D, mut cx ViewCx[T])) {
	cx.update_with(v.model(), data, f)
}

// update_global_with is update_global for a callback that needs data.
pub fn (mut cx Cx) update_global_with[T, D](data D, f fn (mut v T, d D, mut cx ViewCx[T])) {
	cx.app.escaped()
	id := cx.app.globals[typeof[T]().name] or { return }
	cx.update_with(Model[T]{id, cx.app.store.slots[id].gen}, data, f)
}

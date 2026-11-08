// The effect queue: changes are applied immediately, notifications are delivered when
// the outermost update, event or timer callback returns (specs/07-state.md).
module core

struct Observer {
	owner  u32
	target u32
	holder voidptr
	call   fn (holder voidptr, owner voidptr, mut cx Cx) = unsafe { nil }
}

struct Subscription {
	id         u64
	owner      u32
	target     u32
	event_type string
	holder     voidptr
	call       fn (holder voidptr, owner voidptr, event voidptr, mut cx Cx) = unsafe { nil }
}

struct PendingEvent {
	source     u32
	event_type string
	data       voidptr
}

const max_flush_iterations = 10000

fn (mut app App) changed(id u32) {
	if id != 0 && !app.changed_set[id] {
		app.changed_set[id] = true
		app.changed_order << id
	}
}

// flush delivers events, then observers, then invalidates views, until nothing is left.
fn (mut app App) flush() {
	if app.depth > 0 || app.flushing {
		return
	}
	app.flushing = true
	defer {
		app.flushing = false
	}
	for iter := 0; app.pending.len > 0 || app.changed_order.len > 0; iter++ {
		if iter > max_flush_iterations {
			eprintln('quadra: effect cycle detected; dropping ${app.changed_order.len} notifications')
			app.pending.clear()
			app.changed_order.clear()
			app.changed_set.clear()
			break
		}
		if app.pending.len > 0 {
			ev := app.pending[0]
			app.pending.delete(0)
			app.deliver(ev)
			continue
		}
		id := app.changed_order[0]
		app.changed_order.delete(0)
		app.changed_set.delete(id)
		if int(id) >= app.store.slots.len || !app.store.slots[id].alive {
			continue
		}
		for o in app.observers.filter(it.target == id) {
			if ptr, mut cx := app.enter_owned(o.owner) {
				o.call(o.holder, ptr, mut cx)
				app.depth--
			}
		}
		slot := app.store.slots[id]
		if slot.render != unsafe { nil } {
			app.invalidate_view(id)
		}
		for reader, _ in slot.readers {
			app.invalidate_view(reader)
		}
	}
}

fn (mut app App) deliver(ev PendingEvent) {
	for s in app.subs.filter(it.target == ev.source && it.event_type == ev.event_type) {
		if ptr, mut cx := app.enter_owned(s.owner) {
			s.call(s.holder, ptr, ev.data, mut cx)
			app.depth--
		}
	}
}

// enter_owned opens an effect scope for a live owner model and returns its pointer and
// context (none when the owner is gone). The caller runs its callback, then `app.depth--`.
// No closure on this path: V never frees closures, and this runs on every notification.
fn (mut app App) enter_owned(owner u32) ?(voidptr, Cx) {
	if owner != 0 && (int(owner) >= app.store.slots.len || !app.store.slots[owner].alive) {
		return none
	}
	mut ptr := unsafe { nil }
	if owner != 0 {
		ptr = app.store.slots[owner].ptr
	}
	app.depth++
	return ptr, Cx{
		app:    app
		entity: owner
	}
}

fn (mut app App) invalidate_view(id u32) {
	if !app.store.slots[id].alive {
		return
	}
	app.store.slots[id].dirty = true
	if mut w := app.window_of_view(id) {
		w.invalidate()
	} else {
		for mut w in app.windows {
			w.invalidate()
		}
	}
}

fn (mut app App) drop_subscriptions(id u32) {
	app.observers = app.observers.filter(it.owner != id && it.target != id)
	app.subs = app.subs.filter(it.owner != id && it.target != id)
}

// --- typed registration on ViewCx[T] ---
// Holders are built in free generic functions: V mis-instantiates generic struct
// literals inside generic methods of generic receivers.

struct ObserveHolder[T, O] {
	f fn (mut v T, m Model[O], mut cx ViewCx[T]) = unsafe { nil }
	m Model[O]
}

fn call_observer[T, O](holder voidptr, owner voidptr, mut cx Cx) {
	h := unsafe { &ObserveHolder[T, O](holder) }
	mut v := unsafe { &T(owner) }
	mut vcx := ViewCx[T]{
		Cx: cx
	}
	h.f(mut v, h.m, mut vcx)
}

fn make_observer[T, O](owner u32, m Model[O], f fn (mut v T, m Model[O], mut cx ViewCx[T])) Observer {
	return Observer{
		owner:  owner
		target: m.id
		holder: voidptr(&ObserveHolder[T, O]{f, m})
		call:   call_observer[T, O]
	}
}

// observe runs f after every change of m. Owned by the current model.
// A free function: as a method next to `listener[E]`, V would instantiate `listener` with
// every observed model type and emit those structs before their dependencies.
pub fn observe[T, O](mut cx ViewCx[T], m Model[O], f fn (mut v T, m Model[O], mut cx ViewCx[T])) {
	cx.app.escaped_in_render()
	cx.app.observers << make_observer[T, O](cx.entity, m, f)
}

struct SubscribeHolder[T, S, Q] {
	f fn (mut v T, m Model[S], e &Q, mut cx ViewCx[T]) = unsafe { nil }
	m Model[S]
}

fn call_subscriber[T, S, Q](holder voidptr, owner voidptr, event voidptr, mut cx Cx) {
	h := unsafe { &SubscribeHolder[T, S, Q](holder) }
	mut v := unsafe { &T(owner) }
	mut vcx := ViewCx[T]{
		Cx: cx
	}
	h.f(mut v, h.m, unsafe { &Q(event) }, mut vcx)
}

fn make_subscription[T, S, Q](id u64, owner u32, m Model[S], f fn (mut v T, m Model[S], e &Q, mut cx ViewCx[T])) Subscription {
	return Subscription{
		id:         id
		owner:      owner
		target:     m.id
		event_type: typeof[Q]().name
		holder:     voidptr(&SubscribeHolder[T, S, Q]{f, m})
		call:       call_subscriber[T, S, Q]
	}
}

// subscribe runs f for every event of type Q emitted by m. Owned by the current model.
pub fn (mut cx ViewCx[T]) subscribe[S, Q](m Model[S], f fn (mut v T, m Model[S], e &Q, mut cx ViewCx[T])) u64 {
	cx.app.escaped_in_render()
	cx.app.next_sub++
	cx.app.subs << make_subscription[T, S, Q](cx.app.next_sub, cx.entity, m, f)
	return cx.app.next_sub
}

pub fn (mut cx Cx) unsubscribe(id u64) {
	cx.app.subs = cx.app.subs.filter(it.id != id)
}

// emit queues a typed event from the current model to its subscribers.
pub fn (mut cx Cx) emit[E](e E) {
	cx.app.pending << PendingEvent{cx.entity, typeof[E]().name, heap_copy(e)}
	cx.app.flush()
}

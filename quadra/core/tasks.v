// Background tasks owned by models (specs/10): the work runs on a worker thread, the
// completion runs on the main thread with the owner's state, like any other event.
module core

import sync.stdatomic

pub type TaskId = u64

// TaskCx is what the work function sees: a cancel flag it should poll in long loops.
@[heap]
pub struct TaskCx {
mut:
	flag u64
}

pub fn (tc &TaskCx) cancelled() bool {
	return stdatomic.load_u64(&tc.flag) != 0
}

// TaskResult carries the value, or the error message when the work failed.
pub struct TaskResult[R] {
pub:
	value R
	ok    bool
	error string
}

struct TaskEntry {
	owner u32
	tc    &TaskCx
}

// spawn_task runs work on a worker thread; done runs later on the main thread with the
// view or model that spawned it. Releasing the owner cancels the task and drops its result.
// A free function (not a method) because two type parameters on a generic receiver trip V.
pub fn spawn_task[T, R](mut cx ViewCx[T], work fn (tc &TaskCx) !R, done fn (mut v T, r &TaskResult[R], mut cx ViewCx[T])) TaskId {
	mut app := cx.app
	app.escaped_in_render()
	app.next_task++
	id := app.next_task
	tc := &TaskCx{}
	app.tasks[id] = TaskEntry{cx.entity, tc}
	l := listener_of[T, TaskResult[R]](cx.entity, done).any
	app.start_task(id, tc, voidptr(&WorkHolder[R]{work}), run_work[R], l)
	return id
}

// spawn_task_with is spawn_task for work that needs data: work takes it as an argument, so
// the caller makes no closure. V never frees a closure: one per periodic task leaks for good,
// with what it captured (specs/14).
pub fn spawn_task_with[T, D, R](mut cx ViewCx[T], data D, work fn (data D, tc &TaskCx) !R, done fn (mut v T, r &TaskResult[R], mut cx ViewCx[T])) TaskId {
	mut app := cx.app
	app.escaped_in_render()
	app.next_task++
	id := app.next_task
	tc := &TaskCx{}
	app.tasks[id] = TaskEntry{cx.entity, tc}
	l := listener_of[T, TaskResult[R]](cx.entity, done).any
	app.start_task(id, tc, voidptr(&WorkWith[D, R]{data, work}), run_work_with[D, R], l)
	return id
}

struct WorkWith[D, R] {
	d D
	f fn (data D, tc &TaskCx) !R = unsafe { nil }
}

fn run_work_with[D, R](holder voidptr, tc &TaskCx) voidptr {
	h := unsafe { &WorkWith[D, R](holder) }
	zeros := []R{len: 1}
	v := h.f(h.d, tc) or { return voidptr(err_result[R](zeros[0], err.msg())) }
	return voidptr(ok_result[R](v))
}

// The work function is type-erased like listeners (holder + generic fn pointer): V cannot
// spawn generic functions or build generic closures reliably.
struct WorkHolder[R] {
	f fn (tc &TaskCx) !R = unsafe { nil }
}

type WorkCall = fn (holder voidptr, tc &TaskCx) voidptr

fn run_work[R](holder voidptr, tc &TaskCx) voidptr {
	h := unsafe { &WorkHolder[R](holder) }
	zeros := []R{len: 1} // a zero R for the error case
	v := h.f(tc) or { return voidptr(err_result[R](zeros[0], err.msg())) }
	return voidptr(ok_result[R](v))
}

fn ok_result[R](v R) &TaskResult[R] {
	return &TaskResult[R]{
		value: v
		ok:    true
	}
}

fn err_result[R](zero R, msg string) &TaskResult[R] {
	return &TaskResult[R]{
		value: zero
		error: msg
	}
}

fn (mut app App) start_task(id TaskId, tc &TaskCx, holder voidptr, call WorkCall, l AnyListener) {
	// ponytail: one OS thread per task; a fixed worker pool when apps spawn hundreds at once.
	spawn fn (mut app App, id TaskId, tc &TaskCx, holder voidptr, call WorkCall, l AnyListener) {
		result := call(holder, tc)
		app.post_main(task_done, &TaskDone{id, l, result})
	}(mut app, id, tc, holder, call, l)
}

struct TaskDone {
	id     TaskId
	l      AnyListener
	result voidptr
}

fn task_done(mut app App, data voidptr) {
	d := unsafe { &TaskDone(data) }
	app.finish_task(d.id, d.l, d.result)
}

fn (mut app App) finish_task(id TaskId, l AnyListener, result voidptr) {
	e := app.tasks[id] or { return } // cancelled: the result is dropped
	app.tasks.delete(id)
	if e.tc.cancelled() {
		return
	}
	if ptr, mut cx := app.enter_owned(l.view) {
		app.probe_begin()
		l.call(l.holder, ptr, result, mut cx)
		app.probe_end('task completion', l.view)
		app.depth--
	}
	// A completion is a discrete event for its owner, like a click.
	if l.view != 0 && app.store.slots[l.view].alive {
		app.changed(l.view)
	}
	app.flush()
}

// cancel_task sets the task's cancel flag; its completion will not be delivered.
pub fn (mut cx Cx) cancel_task(id TaskId) {
	if e := cx.app.tasks[id] {
		stdatomic.store_u64(&e.tc.flag, 1)
		cx.app.tasks.delete(id)
	}
}

// tasks_running counts tasks whose completion has not been delivered yet.
pub fn (app &App) tasks_running() int {
	return app.tasks.len
}

fn (mut app App) cancel_owned_tasks(owner u32) {
	mut gone := []TaskId{}
	for id, e in app.tasks {
		if e.owner == owner {
			stdatomic.store_u64(&e.tc.flag, 1)
			gone << id
		}
	}
	for id in gone {
		app.tasks.delete(id)
	}
}

// debounce runs f once, ms after the last call with the same key (per owner): autosave,
// search-as-you-type. Each call restarts the wait.
pub fn (mut cx ViewCx[T]) debounce(key string, ms int, f fn (mut v T, mut cx ViewCx[T])) {
	k := '${cx.entity}/${key}'
	if old := cx.app.debounced[k] {
		cx.cancel_timer(old)
	}
	cx.app.debounced[k] = cx.add_timer(ms, false, f)
}

// Background tasks: completion on the main thread, cancellation on release, debounce.
module main

import time
import quadra.core
import quadra.testing
import quadra.ui

struct Loader {
mut:
	results []string
	errors  []string
	saves   int
}

@[markused]
pub fn (mut l Loader) render(mut cx core.ViewCx[Loader]) core.Element {
	return ui.text('loaded: ${l.results.join(',')}')
}

fn load(mut h testing.Harness, v core.View[Loader], input string) core.TaskId {
	mut id := core.TaskId(0)
	mut cx := core.Cx{
		app: h.app
	}
	out := &id
	cx.update_view(v, fn [input, out] (mut l Loader, mut cx core.ViewCx[Loader]) {
		unsafe {
			*out = core.spawn_task[Loader, string](mut cx, fn [input] (tc &core.TaskCx) !string {
				if input == 'bad' {
					return error('cannot read ${input}')
				}
				if input == 'slow' {
					for _ in 0 .. 200 {
						if tc.cancelled() {
							return error('cancelled')
						}
						time.sleep(time.millisecond)
					}
				}
				return input.to_upper()
			}, fn (mut l Loader, r &core.TaskResult[string], mut _ core.ViewCx[Loader]) {
				if r.ok {
					l.results << r.value
				} else {
					l.errors << r.error
				}
			})
		}
	})
	return id
}

fn peek_loader(h &testing.Harness, v core.View[Loader]) &Loader {
	cx := core.Cx{
		app: h.app
	}
	return cx.peek(v.model())
}

fn test_task_completes_on_main_thread_and_rerenders() {
	mut h := testing.new_harness()!
	v := h.open(Loader{})!
	load(mut h, v, 'a')
	load(mut h, v, 'bad')
	assert h.app.tasks_running() == 2
	h.wait_tasks()!
	l := peek_loader(h, v)
	assert l.results == ['A']
	assert l.errors == ['cannot read bad']
	assert h.has_text('loaded: A')
}

fn test_cancel_drops_the_result() {
	mut h := testing.new_harness()!
	v := h.open(Loader{})!
	id := load(mut h, v, 'slow')
	mut cx := core.Cx{
		app: h.app
	}
	cx.cancel_task(id)
	assert h.app.tasks_running() == 0
	time.sleep(20 * time.millisecond) // the worker sees the flag and returns early
	h.wait_tasks()!
	assert peek_loader(h, v).errors.len == 0
	assert peek_loader(h, v).results.len == 0
}

struct Host {
mut:
	child core.View[Loader]
}

@[markused]
pub fn (mut s Host) render(mut cx core.ViewCx[Host]) core.Element {
	if s.child.id == 0 {
		s.child = cx.new_view(Loader{})
	}
	return ui.div() // owns the child without showing it, so releasing it is safe
}

fn test_release_cancels_owned_tasks() {
	mut h := testing.new_harness()!
	hv := h.open(Host{})!
	mut cx := core.Cx{
		app: h.app
	}
	child := cx.peek(hv.model()).child
	load(mut h, child, 'slow')
	assert h.app.tasks_running() == 1
	cx.release(child.model())
	assert h.app.tasks_running() == 0
	h.wait_tasks()!
}

fn test_debounce_runs_once_after_the_last_call() {
	mut h := testing.new_harness()!
	v := h.open(Loader{})!
	mut cx := core.Cx{
		app: h.app
	}
	for _ in 0 .. 3 {
		cx.update_view(v, fn (mut l Loader, mut cx core.ViewCx[Loader]) {
			cx.debounce('save', 500, fn (mut l Loader, mut _ core.ViewCx[Loader]) {
				l.saves++
			})
		})
		h.advance(200)
	}
	assert peek_loader(h, v).saves == 0
	h.advance(400)
	assert peek_loader(h, v).saves == 1
	h.advance(1000)
	assert peek_loader(h, v).saves == 1
}

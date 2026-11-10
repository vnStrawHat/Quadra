// The main-thread queue: worker threads hand results back to the main thread through one
// closure made with the app, not a closure per result. V never frees a closure, and one made
// per task or image kept its result alive for good (specs/14, avoiding closures).
module core

import sync
import quadra.prim

struct MainCall {
	f    fn (mut app App, data prim.Holder) = unsafe { nil }
	data prim.Holder
}

@[heap]
struct MainQueue {
mut:
	mu      &sync.Mutex = sync.new_mutex()
	calls   []MainCall
	pending bool
	drain   fn () = unsafe { nil } // made once in new_app
}

fn (mut app App) init_main_queue() {
	app.mainq = &MainQueue{}
	// Made once per app and posted again and again (the platform's post takes a closure): no leak.
	app.mainq.drain = fn [mut app] () {
		app.drain_main()
	}
}

// post_main_held runs f(app, data) on the main thread; any thread may call it. The way
// worker threads reach the app without a closure; data is a checked holder (specs/14).
pub fn (mut app App) post_main_held(f fn (mut app App, data prim.Holder), data prim.Holder) {
	mut q := app.mainq
	q.mu.@lock()
	q.calls << MainCall{f, data}
	post := !q.pending
	q.pending = true
	q.mu.unlock()
	if post {
		app.plat.post(q.drain)
	}
}

fn (mut app App) drain_main() {
	mut q := app.mainq
	q.mu.@lock()
	calls := q.calls
	q.calls = []MainCall{}
	q.pending = false
	q.mu.unlock()
	for c in calls {
		app.probe_begin()
		c.f(mut app, c.data)
		app.probe_end('main-thread call', 0)
	}
}

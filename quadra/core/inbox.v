// Inbox: a byte stream from any thread into a model (PTY output, log tails). Pushes append
// under a lock and post one drain while none is pending, so a burst of reads costs one
// delivery (specs/17, Delivery).
module core

import sync

// Chunk is what the owner receives: the bytes since the last delivery, and the exit code
// once the producer has finished.
pub struct Chunk {
pub:
	data      []u8
	exited    bool
	exit_code int
}

@[heap]
pub struct Inbox {
mut:
	mu        &sync.Mutex = sync.new_mutex()
	buf       []u8
	pending   bool
	exited    bool
	exit_code int
	delivered bool // the exit was delivered
	app       &App = unsafe { nil }
	l         AnyListener
	task      fn () = unsafe { nil } // the drain, built once: posting allocates nothing
}

// new_inbox makes an inbox whose chunks reach on_chunk on the main thread, with the view or
// model that made it; nothing is delivered once it is released.
pub fn new_inbox[T](mut cx ViewCx[T], on_chunk fn (mut v T, c &Chunk, mut cx ViewCx[T])) &Inbox {
	return make_inbox(cx.app, listener_of[T, Chunk](cx.entity, on_chunk).any)
}

// make_inbox builds the drain closure outside the generic function (V cannot infer T
// around a closure there).
fn make_inbox(app &App, l AnyListener) &Inbox {
	mut ib := &Inbox{
		app: app
		l:   l
	}
	ib.task = fn [mut ib] () {
		ib.drain()
	}
	return ib
}

// push may be called from any thread.
pub fn (mut b Inbox) push(data []u8) {
	b.mu.@lock()
	b.buf << data
	post := !b.pending
	b.pending = true
	b.mu.unlock()
	if post {
		b.post()
	}
}

// finish records the producer's exit code (any thread); it is delivered after the data.
pub fn (mut b Inbox) finish(code int) {
	b.mu.@lock()
	b.exited, b.exit_code = true, code
	post := !b.pending
	b.pending = true
	b.mu.unlock()
	if post {
		b.post()
	}
}

fn (mut b Inbox) post() {
	b.app.plat.post(b.task)
}

fn (mut b Inbox) drain() {
	b.mu.@lock()
	data := b.buf
	b.buf = []u8{}
	b.pending = false
	exited := b.exited && !b.delivered
	if exited {
		b.delivered = true
	}
	b.mu.unlock()
	if data.len == 0 && !exited {
		return
	}
	chunk := &Chunk{data, exited, b.exit_code}
	mut app := b.app
	if ptr, mut cx := app.enter_owned(b.l.view) {
		b.l.call(b.l.holder, ptr, chunk, mut cx)
		app.depth--
	}
	if b.l.view != 0 && app.store.slots[b.l.view].alive {
		app.changed(b.l.view)
	}
	app.flush()
}

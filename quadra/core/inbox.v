// Inbox: a byte stream from any thread into a model (PTY output, log tails). Pushes append
// under a lock and post one drain while none is pending, so a burst of reads costs one
// delivery (specs/17, Delivery).
module core

import sync

// Chunk is what the owner receives: the bytes since the last delivery, and the exit code
// once the producer has finished. `data` is lent for the call: the inbox reuses the buffer.
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
	spare     []u8 // the other buffer, main thread only: drains swap the two, so a steady stream allocates nothing
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
	mut data := unsafe { b.buf }
	b.buf = b.spare
	b.spare = []u8{}
	b.pending = false
	exited := b.exited && !b.delivered
	if exited {
		b.delivered = true
		b.buf = []u8{} // the producer is gone: nothing comes after this
	}
	b.mu.unlock()
	if data.len == 0 && !exited {
		b.spare = data
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
	if keeps_buffer(data.cap, data.len, exited) { // only drains touch spare: no lock
		data.clear()
		b.spare = data
	}
}

// keeps_buffer says whether a drained buffer is kept as the spare. A buffer far bigger than
// what it carried was grown by a burst that is over (`type` of a large file), so it goes to
// the collector, like any buffer once the producer has exited.
fn keeps_buffer(cap int, len int, exited bool) bool {
	return cap <= max_spare && !exited && (cap <= shrink_floor || len >= cap / 8)
}

// max_spare is the largest buffer a drain keeps for reuse; a bigger burst is left to the GC.
const max_spare = 4 << 20

// shrink_floor: buffers up to this size are always kept.
const shrink_floor = 256 << 10

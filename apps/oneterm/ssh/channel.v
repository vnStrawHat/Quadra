// One SSH channel: a buffered byte pipe with window flow control (RFC 4254 section 5).
module ssh

import io
import sync
import time

@[heap]
struct Channel {
mut:
	conn           &Conn
	id             u32
	remote_id      u32
	window_initial u32
	consumed       u32
	mu             &sync.Mutex = sync.new_mutex()
	cond           &sync.Cond  = unsafe { nil }
	rx             [][]u8
	rx_off         int
	remote_window  u64
	max_packet     u32
	eof            bool
	closed         bool
	open_wait      &Waiter = &Waiter{}
	req_waits      []&Waiter
	exit_code      int = -1
	has_exit       bool
	traffic        &Traffic = unsafe { nil }
}

fn new_channel(conn &Conn, id u32, window u32) &Channel {
	mut ch := &Channel{
		conn:           conn
		id:             id
		window_initial: window
		max_packet:     default_max_packet
	}
	ch.cond = sync.new_cond(ch.mu)
	return ch
}

fn (mut ch Channel) confirmed(remote u32, window u32, packet u32) {
	ch.mu.@lock()
	ch.remote_id = remote
	ch.remote_window = u64(window)
	ch.max_packet = if packet < 1024 || packet > default_max_packet {
		default_max_packet
	} else {
		packet
	}
	ch.mu.unlock()
	ch.open_wait.finish(true, [])
}

fn (mut ch Channel) window_add(n u32) {
	ch.mu.@lock()
	ch.remote_window += u64(n)
	ch.cond.broadcast()
	ch.mu.unlock()
}

fn (mut ch Channel) receive(data []u8) {
	if data.len == 0 {
		return
	}
	ch.mu.@lock()
	ch.rx << data
	ch.cond.broadcast()
	ch.mu.unlock()
}

fn (mut ch Channel) got_eof() {
	ch.mu.@lock()
	ch.eof = true
	ch.cond.broadcast()
	ch.mu.unlock()
}

// got_close replies to a remote close; the channel stays readable until drained.
fn (mut ch Channel) got_close() ! {
	ch.mu.@lock()
	was_closed := ch.closed
	ch.closed = true
	ch.eof = true
	ch.cond.broadcast()
	ch.mu.unlock()
	ch.finish_requests()
	if !was_closed {
		ch.send_close()!
	}
}

fn (mut ch Channel) send_close() ! {
	mut w := new_writer(8)
	w.put_u8(msg_channel_close)
	w.put_u32(ch.remote_id)
	ch.conn.tr.send(w.buf)!
}

// conn_lost wakes everything when the connection itself is gone.
fn (mut ch Channel) conn_lost() {
	ch.mu.@lock()
	ch.closed = true
	ch.eof = true
	ch.cond.broadcast()
	ch.mu.unlock()
	ch.open_wait.finish(false, ch.conn.reason.bytes())
	ch.finish_requests()
}

fn (mut ch Channel) finish_requests() {
	ch.mu.@lock()
	waits := ch.req_waits.clone()
	ch.req_waits = []
	ch.mu.unlock()
	for w in waits {
		mut waiter := unsafe { w }
		waiter.finish(false, [])
	}
}

fn (mut ch Channel) request_reply(ok bool) {
	ch.mu.@lock()
	if ch.req_waits.len == 0 {
		ch.mu.unlock()
		return
	}
	mut w := ch.req_waits.first()
	ch.req_waits.delete(0)
	ch.mu.unlock()
	w.finish(ok, [])
}

fn (mut ch Channel) incoming_request(name string, mut r Reader) {
	if name == 'exit-status' {
		code := r.get_u32() or { return }
		ch.mu.@lock()
		ch.exit_code = int(code)
		ch.has_exit = true
		ch.cond.broadcast()
		ch.mu.unlock()
	} else if name == 'exit-signal' {
		ch.mu.@lock()
		ch.has_exit = true
		ch.cond.broadcast()
		ch.mu.unlock()
	}
}

// read returns the next bytes, or io.Eof once the peer ended and the buffer is drained.
fn (mut ch Channel) read(mut buf []u8) !int {
	ch.mu.@lock()
	for ch.rx.len == 0 && !ch.eof {
		ch.cond.wait()
	}
	if ch.rx.len == 0 {
		ch.mu.unlock()
		return io.Eof{}
	}
	head := ch.rx[0]
	avail := head.len - ch.rx_off
	n := if avail < buf.len { avail } else { buf.len }
	copy(mut buf, head[ch.rx_off..ch.rx_off + n])
	ch.rx_off += n
	if ch.rx_off == head.len {
		ch.rx.delete(0)
		ch.rx_off = 0
	}
	ch.consumed += u32(n)
	mut grant := u32(0)
	if ch.consumed >= ch.window_initial / 2 && !ch.closed {
		grant = ch.consumed
		ch.consumed = 0
	}
	ch.mu.unlock()
	if ch.traffic != unsafe { nil } {
		ch.traffic.add_rx(n)
	}
	if grant > 0 {
		mut w := new_writer(12)
		w.put_u8(msg_window_adjust)
		w.put_u32(ch.remote_id)
		w.put_u32(grant)
		ch.conn.tr.send(w.buf) or {}
	}
	return n
}

// read_full fills buf or fails; used for framed protocols.
fn (mut ch Channel) read_full(mut buf []u8) ! {
	mut got := 0
	for got < buf.len {
		n := ch.read(mut buf[got..])!
		got += n
	}
}

// write sends everything, waiting for window space.
fn (mut ch Channel) write(data []u8) ! {
	mut pos := 0
	for pos < data.len {
		ch.mu.@lock()
		for ch.remote_window == 0 && !ch.closed {
			ch.cond.wait()
		}
		if ch.closed {
			ch.mu.unlock()
			return error('channel closed')
		}
		mut n := data.len - pos
		if u64(n) > ch.remote_window {
			n = int(ch.remote_window)
		}
		if n > int(ch.max_packet) {
			n = int(ch.max_packet)
		}
		ch.remote_window -= u64(n)
		ch.mu.unlock()
		mut w := new_writer(n + 12)
		w.put_u8(msg_channel_data)
		w.put_u32(ch.remote_id)
		w.put_bytes(data[pos..pos + n])
		ch.conn.tr.send(w.buf)!
		pos += n
		if ch.traffic != unsafe { nil } {
			ch.traffic.add_tx(n)
		}
	}
}

// request sends a channel request; with want_reply it waits and returns the answer.
fn (mut ch Channel) request(name string, data []u8, want_reply bool) !bool {
	mut w := new_writer(name.len + data.len + 12)
	w.put_u8(msg_channel_request)
	w.put_u32(ch.remote_id)
	w.put_str(name)
	w.put_bool(want_reply)
	w.put_raw(data)
	if !want_reply {
		ch.conn.tr.send(w.buf)!
		return true
	}
	mut waiter := &Waiter{}
	ch.mu.@lock()
	if ch.closed {
		ch.mu.unlock()
		return error('channel closed')
	}
	ch.req_waits << waiter
	ch.mu.unlock()
	ch.conn.tr.send(w.buf)!
	return waiter.wait()
}

// send_eof tells the peer no more data will come.
fn (mut ch Channel) send_eof() ! {
	mut w := new_writer(8)
	w.put_u8(msg_channel_eof)
	w.put_u32(ch.remote_id)
	ch.conn.tr.send(w.buf)!
}

// close sends a close once; safe to call more than once.
fn (mut ch Channel) close() {
	ch.mu.@lock()
	was := ch.closed
	ch.closed = true
	ch.cond.broadcast()
	ch.mu.unlock()
	if !was {
		ch.send_close() or {}
	}
}

// wait_exit waits briefly for the exit status after the data ends.
fn (mut ch Channel) wait_exit(ms int) int {
	deadline := time.now().add(ms * time.millisecond)
	for time.now() < deadline {
		ch.mu.@lock()
		done := ch.has_exit || ch.closed
		code := ch.exit_code
		ch.mu.unlock()
		if done {
			return code
		}
		time.sleep(10 * time.millisecond)
	}
	ch.mu.@lock()
	defer {
		ch.mu.unlock()
	}
	return ch.exit_code
}

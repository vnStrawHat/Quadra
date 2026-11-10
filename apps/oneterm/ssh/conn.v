// The connection protocol (RFC 4254): channels, global requests, the reader thread (specs/23 D3)
// and the keepalive thread.
module ssh

import sync
import time

const msg_global_request = u8(80)
const msg_request_success = u8(81)
const msg_request_failure = u8(82)
const msg_channel_open = u8(90)
const msg_channel_open_confirm = u8(91)
const msg_channel_open_failure = u8(92)
const msg_window_adjust = u8(93)
const msg_channel_data = u8(94)
const msg_channel_extended = u8(95)
const msg_channel_eof = u8(96)
const msg_channel_close = u8(97)
const msg_channel_request = u8(98)
const msg_channel_success = u8(99)
const msg_channel_failure = u8(100)

const default_window = u32(2 * 1024 * 1024)
const default_max_packet = u32(32768)

// Waiter is a one-shot reply slot: a reply from the reader thread, or a close.
@[heap]
struct Waiter {
mut:
	sem     &sync.Semaphore = sync.new_semaphore()
	ok      bool
	payload []u8
}

fn (mut w Waiter) finish(ok bool, payload []u8) {
	w.ok = ok
	w.payload = payload
	w.sem.post()
}

fn (mut w Waiter) wait() bool {
	w.sem.wait()
	return w.ok
}

// GlobalWait is one global request waiting for its reply.
struct GlobalWait {
mut:
	waiter &Waiter = unsafe { nil } // nil: a keepalive, the reply is discarded
}

// Traffic counts payload bytes for the bandwidth indicator.
@[heap]
pub struct Traffic {
mut:
	mu &sync.Mutex = sync.new_mutex()
	rx u64
	tx u64
}

fn (mut t Traffic) add_rx(n int) {
	t.mu.@lock()
	t.rx += u64(n)
	t.mu.unlock()
}

fn (mut t Traffic) add_tx(n int) {
	t.mu.@lock()
	t.tx += u64(n)
	t.mu.unlock()
}

// snapshot returns (received, sent).
pub fn (mut t Traffic) snapshot() (u64, u64) {
	t.mu.@lock()
	defer {
		t.mu.unlock()
	}
	return t.rx, t.tx
}

@[heap]
struct Conn {
mut:
	tr       &Transport
	mu       &sync.Mutex = sync.new_mutex()
	channels map[u32]&Channel
	next_id  u32
	globals  []GlobalWait
	closed   bool
	reason   string
	last_rx  time.Time
	missed   int
	stop     &sync.Semaphore = sync.new_semaphore()
	// What channels the server opens go to, and who hears the end: references, not closures
	// (no closures in apps, specs/14).
	forwards   &Forwards = unsafe { nil } // remote forwards: forwarded-tcpip channels
	agent_pipe string // agent forwarding on: auth-agent channels bridge to this pipe
	agent_on   bool
	owner      &Session = unsafe { nil } // closed when the connection drops
}

fn new_conn(tr &Transport) &Conn {
	return &Conn{
		tr:      tr
		last_rx: time.now()
	}
}

// start spawns the reader, and the keepalive thread when enabled.
fn (mut c Conn) start(ka Keepalive) {
	spawn c.reader()
	if ka.enabled {
		spawn c.keepalive_loop(ka.seconds, ka.max)
	}
}

fn (mut c Conn) reader() {
	mut reason := ''
	for {
		pkt := c.tr.next_packet() or {
			reason = err.msg()
			break
		}
		c.mu.@lock()
		c.last_rx = time.now()
		c.missed = 0
		c.mu.unlock()
		c.dispatch(pkt) or {
			reason = err.msg()
			break
		}
	}
	c.shutdown(reason)
}

// close disconnects politely; shutdown ends everything that waits on this connection.
fn (mut c Conn) close() {
	mut w := new_writer(32)
	w.put_u8(msg_disconnect)
	w.put_u32(11)
	w.put_str('closed by client')
	w.put_str('')
	c.tr.send(w.buf) or {}
	c.shutdown('closed')
}

fn (mut c Conn) shutdown(reason string) {
	c.mu.@lock()
	if c.closed {
		c.mu.unlock()
		return
	}
	c.closed = true
	c.reason = reason
	channels := c.channels.values()
	globals := c.globals.clone()
	c.globals = []
	c.mu.unlock()
	c.stop.post()
	c.tr.close()
	for ch in channels {
		mut channel := unsafe { ch }
		channel.conn_lost()
	}
	for g in globals {
		if g.waiter != unsafe { nil } {
			mut w := g.waiter
			w.finish(false, [])
		}
	}
	if c.owner != unsafe { nil } {
		mut s := c.owner
		spawn s.close()
	}
}

fn (mut c Conn) keepalive_loop(seconds int, max int) {
	interval := seconds * time.second
	for {
		if c.stop.timed_wait(interval) {
			return
		}
		c.mu.@lock()
		idle := time.since(c.last_rx) >= interval
		mut lost := false
		if idle {
			c.missed++
			lost = c.missed > max
		}
		c.mu.unlock()
		if lost {
			c.shutdown('keepalive timeout')
			return
		}
		if idle {
			c.global_request('keepalive@openssh.com', [], false, true) or { return }
		}
	}
}

fn (mut c Conn) global_request(name string, data []u8, want_reply bool, discard bool) !&Waiter {
	mut w := new_writer(name.len + data.len + 8)
	w.put_u8(msg_global_request)
	w.put_str(name)
	w.put_bool(want_reply || discard)
	w.put_raw(data)
	mut waiter := &Waiter{}
	c.mu.@lock()
	if c.closed {
		c.mu.unlock()
		return error('connection closed')
	}
	if want_reply || discard {
		if discard {
			c.globals << GlobalWait{}
		} else {
			c.globals << GlobalWait{
				waiter: waiter
			}
		}
	}
	c.mu.unlock()
	c.tr.send(w.buf)!
	return waiter
}

fn (mut c Conn) channel(id u32) ?&Channel {
	c.mu.@lock()
	defer {
		c.mu.unlock()
	}
	return c.channels[id] or { return none }
}

fn (mut c Conn) remove_channel(id u32) {
	c.mu.@lock()
	c.channels.delete(id)
	c.mu.unlock()
}

// open_channel opens a client channel and waits for the confirmation.
fn (mut c Conn) open_channel(kind string, extra []u8, window u32) !&Channel {
	c.mu.@lock()
	if c.closed {
		c.mu.unlock()
		return error('connection closed: ${c.reason}')
	}
	id := c.next_id
	c.next_id++
	mut ch := new_channel(c, id, window)
	c.channels[id] = ch
	c.mu.unlock()
	mut w := new_writer(extra.len + 32)
	w.put_u8(msg_channel_open)
	w.put_str(kind)
	w.put_u32(id)
	w.put_u32(window)
	w.put_u32(default_max_packet)
	w.put_raw(extra)
	c.tr.send(w.buf)!
	if !ch.open_wait.wait() {
		c.remove_channel(id)
		return error('channel open refused: ${ch.open_wait.payload.bytestr()}')
	}
	return ch
}

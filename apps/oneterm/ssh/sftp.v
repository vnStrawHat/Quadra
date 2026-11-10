// The SFTP client: one demux thread matches replies to requests, so any worker may call it.
module ssh

import sync

const sftp_window = u32(8 * 1024 * 1024)
const sftp_chunk_len = u32(255 * 1024)
const sftp_max_reply = 2 * 1024 * 1024

// Sftp is the handle the UI holds; its methods block, so call them on a worker thread.
@[heap]
pub struct Sftp {
mut:
	ch        &Channel
	mu        &sync.Mutex = sync.new_mutex()
	send_mu   &sync.Mutex = sync.new_mutex()
	pending   map[u32]&Waiter
	next_id   u32
	alive     bool = true
	closed    bool
	max_read  u32 = sftp_chunk_len
	max_write u32 = sftp_chunk_len
	max_open  u64
	open_now  u64
	active    map[u64]&Cancel
	uid_names map[u32]string
	gid_names map[u32]string
	nonce     u64
}

fn be32_of(b []u8) u32 {
	return u32(b[0]) << 24 | u32(b[1]) << 16 | u32(b[2]) << 8 | u32(b[3])
}

fn new_sftp(mut ch Channel) !&Sftp {
	mut s := &Sftp{
		ch: ch
	}
	mut init := new_writer(16)
	init.put_u32(5)
	init.put_u8(fx_init)
	init.put_u32(3)
	ch.write(init.buf)!
	has_limits := parse_version(read_frame(mut ch)!)!
	spawn s.demux()
	if has_limits {
		s.query_limits()
	}
	spawn s.load_id_names()
	return s
}

// parse_version reads SSH_FXP_VERSION; true when the server has limits@openssh.com.
fn parse_version(body []u8) !bool {
	if body.len < 5 || body[0] != fx_version { // the type and the u32 version
		return error('expected SFTP VERSION, got ${body.len} bytes')
	}
	mut has_limits := false
	mut r := new_reader(body[5..])
	for r.left() > 0 {
		name := r.get_str()!
		r.get_bytes()!
		if name == 'limits@openssh.com' {
			has_limits = true
		}
	}
	return has_limits
}

fn read_frame(mut ch Channel) ![]u8 {
	mut header := []u8{len: 4}
	ch.read_full(mut header)!
	n := int(be32_of(header))
	if n < 1 || n > sftp_max_reply {
		return error('bad SFTP frame length ${n}')
	}
	mut body := []u8{len: n}
	ch.read_full(mut body)!
	return body
}

fn (mut s Sftp) demux() {
	for {
		body := read_frame(mut s.ch) or { break }
		if body.len < 5 {
			break
		}
		id := be32_of(body[1..5])
		s.mu.@lock()
		mut w := s.pending[id] or {
			s.mu.unlock()
			continue // a request forgotten after a cancel
		}
		s.pending.delete(id)
		s.mu.unlock()
		w.finish(true, body)
	}
	s.mu.@lock()
	s.alive = false
	waits := s.pending.values()
	s.pending.clear()
	s.mu.unlock()
	for w in waits {
		mut waiter := unsafe { w }
		waiter.finish(false, [])
	}
}

// start sends a request without waiting; used directly by the pipelined transfers.
fn (mut s Sftp) start(kind u8, payload []u8) !(u32, &Waiter) {
	mut w := &Waiter{}
	s.mu.@lock()
	if !s.alive {
		s.mu.unlock()
		return error('SFTP session is closed')
	}
	s.next_id++
	id := s.next_id
	s.pending[id] = w
	s.mu.unlock()
	mut frame := new_writer(payload.len + 16)
	frame.put_u32(u32(payload.len + 5))
	frame.put_u8(kind)
	frame.put_u32(id)
	frame.put_raw(payload)
	s.send_mu.@lock()
	s.ch.write(frame.buf) or {
		s.send_mu.unlock()
		s.forget(id)
		return err
	}
	s.send_mu.unlock()
	return id, w
}

// forget drops a request whose reply nobody will read.
fn (mut s Sftp) forget(id u32) {
	s.mu.@lock()
	s.pending.delete(id)
	s.mu.unlock()
}

fn await(mut w Waiter) ![]u8 {
	if !w.wait() {
		return SftpError{
			status:  .connection_lost
			message: 'SFTP connection lost'
		}
	}
	return w.payload
}

// call sends a request and waits for its reply.
fn (mut s Sftp) call(kind u8, payload []u8) ![]u8 {
	_, mut w := s.start(kind, payload)!
	return await(mut w)
}

// call_status is a request answered by a status.
fn (mut s Sftp) call_status(kind u8, payload []u8) ! {
	body := s.call(kind, payload)!
	if body[0] != fx_status {
		return error('unexpected SFTP reply ${body[0]}')
	}
	status_error(body)!
}

fn path_payload(path string) []u8 {
	mut w := new_writer(path.len + 8)
	w.put_str(path)
	return w.buf
}

// query_limits reads limits@openssh.com and clamps the transfer sizes.
fn (mut s Sftp) query_limits() {
	mut w := new_writer(32)
	w.put_str('limits@openssh.com')
	body := s.call(fx_extended, w.buf) or { return }
	if body[0] != fx_extended_reply {
		return
	}
	mut r := new_reader(body[5..])
	r.get_u64() or { return }
	max_read := r.get_u64() or { return }
	max_write := r.get_u64() or { return }
	max_open := r.get_u64() or { return }
	s.mu.@lock()
	if max_read > 0 && max_read < u64(s.max_read) {
		s.max_read = u32(max_read)
	}
	if max_write > 0 && max_write < u64(s.max_write) {
		s.max_write = u32(max_write)
	}
	s.max_open = max_open
	s.mu.unlock()
}

// alive reports whether the session can still serve requests.
pub fn (mut s Sftp) alive() bool {
	s.mu.@lock()
	defer {
		s.mu.unlock()
	}
	return s.alive
}

// close cancels running transfers and closes the channel.
pub fn (mut s Sftp) close() {
	s.mu.@lock()
	if s.closed {
		s.mu.unlock()
		return
	}
	s.closed = true
	s.alive = false
	active := s.active.values()
	s.mu.unlock()
	for c in active {
		mut token := unsafe { c }
		token.cancel()
	}
	s.ch.close()
}

// register records a transfer id; a duplicate of a running one is refused.
fn (mut s Sftp) register(id u64) !&Cancel {
	s.mu.@lock()
	defer {
		s.mu.unlock()
	}
	if id in s.active {
		return error('duplicate active transfer id: ${id}')
	}
	token := new_cancel()
	s.active[id] = token
	return token
}

fn (mut s Sftp) unregister(id u64) {
	s.mu.@lock()
	s.active.delete(id)
	s.mu.unlock()
}

// cancel_transfer stops the transfer with this id; unknown ids are ignored.
pub fn (mut s Sftp) cancel_transfer(id u64) {
	s.mu.@lock()
	mut token := s.active[id] or {
		s.mu.unlock()
		return
	}
	s.mu.unlock()
	token.cancel()
}

fn (mut s Sftp) next_nonce() u64 {
	s.mu.@lock()
	defer {
		s.mu.unlock()
	}
	s.nonce++
	return s.nonce
}

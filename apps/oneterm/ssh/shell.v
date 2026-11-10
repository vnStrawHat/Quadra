// The shell channel behind platform.Pty: output pump, write queue, coalesced resize (specs/23 D7).
module ssh

import sync
import quadra.platform

const write_budget = 4 * 1024 * 1024

@[heap]
pub struct ShellPty {
mut:
	session  &Session = unsafe { nil }
	ch       &Channel
	mu       &sync.Mutex     = sync.new_mutex()
	wake     &sync.Semaphore = sync.new_semaphore()
	queue    [][]u8
	queued   int
	resize_c int
	resize_r int
	resize   bool
	closing  bool
}

fn new_shell_pty(ch &Channel) &ShellPty {
	return &ShellPty{
		ch: ch
	}
}

// start spawns the two worker threads; attach calls it once.
fn (mut p ShellPty) start(opts platform.PtyOptions) {
	spawn p.pump(opts.on_data, opts.on_exit)
	spawn p.writer()
}

// write never blocks: it fails when the 4 MiB budget is full or the shell is closing.
pub fn (mut p ShellPty) write(data []u8) ! {
	p.mu.@lock()
	defer {
		p.mu.unlock()
	}
	if p.closing {
		return error('session closed')
	}
	if data.len == 0 {
		return
	}
	if p.queued + data.len > write_budget {
		return error('SSH write queue is full')
	}
	p.queue << data.clone()
	p.queued += data.len
	p.wake.post()
}

// resize keeps only the latest size.
pub fn (mut p ShellPty) resize(cols int, rows int) ! {
	p.mu.@lock()
	defer {
		p.mu.unlock()
	}
	if p.closing {
		return error('session closed')
	}
	p.resize_c = cols
	p.resize_r = rows
	p.resize = true
	p.wake.post()
}

// close ends the session; the on_exit callback still fires once.
pub fn (mut p ShellPty) close() {
	p.mu.@lock()
	if p.closing {
		p.mu.unlock()
		return
	}
	p.closing = true
	p.mu.unlock()
	p.wake.post()
	spawn p.close_session()
}

fn (mut p ShellPty) close_session() {
	p.ch.close()
	if p.session != unsafe { nil } {
		mut s := p.session
		s.close()
	}
}

fn (mut p ShellPty) writer() {
	for {
		p.wake.wait()
		p.mu.@lock()
		mut closing := p.closing
		mut item := []u8{}
		mut have_data := false
		if p.queue.len > 0 {
			item = p.queue.first()
			p.queue.delete(0)
			p.queued -= item.len
			have_data = true
		}
		has_resize := p.resize
		cols, rows := p.resize_c, p.resize_r
		p.resize = false
		p.mu.unlock()
		if closing {
			return
		}
		if has_resize {
			mut w := new_writer(16)
			w.put_u32(u32(cols))
			w.put_u32(u32(rows))
			w.put_u32(0)
			w.put_u32(0)
			p.ch.request('window-change', w.buf, false) or {}
		}
		if have_data {
			p.ch.write(item) or { return }
		}
	}
}

fn (mut p ShellPty) pump(on_data fn (data []u8), on_exit fn (code int)) {
	mut buf := []u8{len: 64 * 1024}
	for {
		n := p.ch.read(mut buf) or { break }
		if on_data != unsafe { nil } {
			on_data(buf[..n].clone())
		}
	}
	code := p.ch.wait_exit(500)
	p.mu.@lock()
	p.closing = true
	p.mu.unlock()
	p.wake.post()
	if p.session != unsafe { nil } {
		mut s := p.session
		s.close()
	}
	if on_exit != unsafe { nil } {
		on_exit(code)
	}
}

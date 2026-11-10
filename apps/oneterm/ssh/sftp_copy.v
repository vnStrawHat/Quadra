// The transfer engine: 16 reads or 8 writes in flight over one handle, sequential per file.
module ssh

import io
import os

const reads_in_flight = 16
const writes_in_flight = 8

pub enum TransferKind {
	progress    // fraction 0.0 .. 1.0
	discovering // files found so far while listing a folder
	cancelled
}

pub struct TransferEvent {
pub:
	kind     TransferKind
	fraction f64
	files    int
}

// OnTransfer receives a transfer's events on the worker thread: f(ctx, event). Data rides in
// ctx, so no closure is made per transfer (no closures in apps, specs/14). Zero = no events.
pub struct OnTransfer {
pub:
	ctx voidptr
	f   fn (ctx voidptr, e TransferEvent) = unsafe { nil }
}

// TreeProgress is the running total of a folder transfer.
@[heap]
struct TreeProgress {
mut:
	done     u64
	reported f64
}

// Reporter reports the running total as progress, once per chunk of bytes advanced: of one
// file (tree nil), or of a folder's file that started at `start` bytes into the tree.
struct Reporter {
mut:
	on_event OnTransfer
	total    u64
	start    u64
	tree     &TreeProgress = unsafe { nil }
	copied   u64
	reported u64
}

fn (mut p Reporter) add(n u64) {
	p.copied += n
	if p.copied - p.reported >= u64(sftp_chunk_len) {
		p.reported = p.copied
		p.on_bytes(p.copied)
	}
}

fn (mut p Reporter) finish() {
	if p.copied > p.reported {
		p.on_bytes(p.copied)
	}
}

fn (mut p Reporter) on_bytes(n u64) {
	if p.tree == unsafe { nil } {
		emit(p.on_event, .progress, fraction_of(n, p.total), 0)
		return
	}
	mut tree := p.tree
	tree.done = p.start + n
	if p.total > 0 {
		fr := fraction_of(tree.done, p.total)
		if fr > tree.reported {
			tree.reported = fr
			emit(p.on_event, .progress, fr, 0)
		}
	}
}

struct PendingRead {
mut:
	offset u64
	len    u32
	id     u32
	waiter &Waiter
}

fn (mut s Sftp) send_read(handle []u8, offset u64, n u32) !PendingRead {
	mut w := new_writer(handle.len + 24)
	w.put_bytes(handle)
	w.put_u64(offset)
	w.put_u32(n)
	id, waiter := s.start(fx_read, w.buf)!
	return PendingRead{offset, n, id, waiter}
}

// pull copies a remote handle to a local file, up to limit bytes (0: to EOF).
fn (mut s Sftp) pull(handle []u8, mut out os.File, limit u64, mut cancel Cancel, mut rep Reporter) ! {
	end := if limit == 0 { max_u64 } else { limit }
	chunk := s.max_read
	mut queue := []PendingRead{}
	mut offset := u64(0)
	mut eof := false
	defer {
		for q in queue {
			s.forget(q.id)
		}
	}
	for {
		if cancel.is_cancelled() {
			return Cancelled{}
		}
		for !eof && queue.len < reads_in_flight && offset < end {
			n := u32(if end - offset < u64(chunk) { end - offset } else { u64(chunk) })
			queue << s.send_read(handle, offset, n)!
			offset += u64(n)
		}
		if queue.len == 0 {
			break
		}
		mut head := queue.first()
		queue.delete(0)
		body := await(mut head.waiter)!
		if body[0] == fx_status {
			status_error(body) or {
				if err is SftpError && err.status == .eof {
					eof = true
					continue
				}
				return err
			}
			eof = true
			continue
		}
		if body[0] != fx_data {
			return error('unexpected SFTP reply ${body[0]}')
		}
		mut r := new_reader(body[5..])
		data := r.get_bytes()!
		if data.len > int(head.len) {
			return error('the server sent more data than requested')
		}
		out.write(data) or { return error('write local file: ${err}') }
		rep.add(u64(data.len))
		if data.len < int(head.len) && data.len > 0 {
			// a short read: ask for the remainder before anything queued after it
			queue.prepend(s.send_read(handle, head.offset + u64(data.len), head.len - u32(data.len))!)
		}
		if data.len == 0 {
			eof = true
		}
	}
	rep.finish()
}

struct PendingWrite {
mut:
	id     u32
	waiter &Waiter
	n      u64
}

// push copies a local file to a remote handle.
fn (mut s Sftp) push(mut input os.File, handle []u8, mut cancel Cancel, mut rep Reporter) ! {
	chunk := int(s.max_write)
	mut queue := []PendingWrite{}
	mut offset := u64(0)
	mut eof := false
	mut buf := []u8{len: chunk}
	defer {
		for q in queue {
			s.forget(q.id)
		}
	}
	for {
		if cancel.is_cancelled() {
			return Cancelled{}
		}
		for !eof && queue.len < writes_in_flight {
			n := input.read(mut buf) or {
				if err is io.Eof || err is os.Eof {
					0
				} else {
					return error('read local file: ${err}')
				}
			}
			if n == 0 {
				eof = true
				break
			}
			mut w := new_writer(handle.len + n + 16)
			w.put_bytes(handle)
			w.put_u64(offset)
			w.put_bytes(buf[..n])
			id, waiter := s.start(fx_write, w.buf)!
			queue << PendingWrite{id, waiter, u64(n)}
			offset += u64(n)
		}
		if queue.len == 0 {
			break
		}
		mut head := queue.first()
		queue.delete(0)
		body := await(mut head.waiter)!
		if body[0] != fx_status {
			return error('unexpected SFTP reply ${body[0]}')
		}
		status_error(body)!
		rep.add(head.n)
	}
	rep.finish()
}

fn emit(on_event OnTransfer, kind TransferKind, fraction f64, files int) {
	if on_event.f != unsafe { nil } {
		on_event.f(on_event.ctx, TransferEvent{kind, fraction, files})
	}
}

fn fraction_of(done u64, total u64) f64 {
	if total == 0 {
		return 1.0
	}
	f := f64(done) / f64(total)
	return if f > 1.0 { 1.0 } else { f }
}

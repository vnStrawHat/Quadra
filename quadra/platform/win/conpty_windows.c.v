// ConPTY: a pseudo console with a reader thread (specs/17). The pseudo console functions and
// the attribute-list helpers are loaded at run time (conpty_host_windows.c.v picks the host).
module win

import os
import sync
import quadra.platform
import quadra.prim

type FnCreatePC = fn (size u32, input voidptr, output voidptr, flags u32, out &voidptr) int

type FnResizePC = fn (hpc voidptr, size u32) int

type FnClosePC = fn (hpc voidptr)

type FnInitAttrs = fn (list voidptr, count u32, flags u32, size &usize) int

type FnUpdateAttr = fn (list voidptr, flags u32, attr usize, value voidptr, size usize, prev voidptr, ret voidptr) int

const proc_thread_attribute_pseudoconsole = usize(0x00020016)
const extended_startupinfo_present = u32(0x00080000)
const create_unicode_environment = u32(0x00000400)
const startf_usestdhandles = u32(0x00000100)
const duplicate_same_access = u32(2)
const wait_timeout = 0x102
// child_exit_grace: how long a closed session's program may take to exit before it is
// terminated. A program whose start-up had not finished when its pseudo console closed never
// exits on its own (it never sees the host's exit request), and keeps its console host alive.
const child_exit_grace_ms = 2000

fn C.TerminateProcess(process voidptr, code u32) int
fn C.DuplicateHandle(src_process voidptr, src voidptr, dst_process voidptr, dst &voidptr, access u32, inherit int, options u32) int

// StartupInfoEx mirrors STARTUPINFOEXW (x64 layout).
struct StartupInfoEx {
mut:
	cb          u32
	reserved    voidptr
	desktop     voidptr
	title       voidptr
	dims        [8]u32 // position, size, count chars, fill attribute, flags (index 7)
	show_window u16
	cb_res2     u16
	res2        voidptr
	std_in      voidptr
	std_out     voidptr
	std_err     voidptr
	attrs       voidptr
}

struct ProcessInfo {
mut:
	process voidptr
	thread  voidptr
	pid     u32
	tid     u32
}

struct PcApi {
	create FnCreatePC   = unsafe { nil }
	resize FnResizePC   = unsafe { nil }
	close  FnClosePC    = unsafe { nil }
	init   FnInitAttrs  = unsafe { nil }
	update FnUpdateAttr = unsafe { nil }
	host   ConptyHost
}

@[heap]
pub struct ConPty {
mut:
	api     PcApi
	hpc     voidptr
	input   voidptr // our end: we write keys here
	output  voidptr // our end: the reader reads the screen output here
	process voidptr
	mu      &sync.Mutex = sync.new_mutex()
	closed  bool
	queue   [][]u8 // for the writer thread
	queued  int
	wake    &sync.Semaphore = sync.new_semaphore()
}

fn coord(cols int, rows int) u32 {
	return u32(u16(cols)) | (u32(u16(rows)) << 16)
}

pub fn spawn_conpty(opts platform.PtyOptions) !platform.Pty {
	api := pc_api() or { return error('ConPTY needs Windows 10 1809 or later: ${err}') }
	mut in_read, mut in_write := unsafe { nil }, unsafe { nil }
	mut out_read, mut out_write := unsafe { nil }, unsafe { nil }
	if !C.CreatePipe(&in_read, &in_write, unsafe { nil }, 1 << 20)
		|| !C.CreatePipe(&out_read, &out_write, unsafe { nil }, 1 << 20) {
		return error('CreatePipe failed: ${C.GetLastError()}')
	}
	mut hpc := unsafe { nil }
	mut flags := u32(0) // an older inbox host may reject flags it does not know
	if api.host == .bundled {
		flags = pseudoconsole_glyph_width_wcswidth
	}
	hr := api.create(coord(opts.cols, opts.rows), in_read, out_write, flags, &hpc)
	C.CloseHandle(in_read)
	C.CloseHandle(out_write)
	if hr != 0 {
		C.CloseHandle(in_write)
		C.CloseHandle(out_read)
		return error('CreatePseudoConsole failed: 0x${u32(hr).hex()}')
	}
	mut size := usize(0)
	api.init(unsafe { nil }, 1, 0, &size)
	mut list := []u8{len: int(size)}
	api.init(list.data, 1, 0, &size)
	api.update(list.data, 0, proc_thread_attribute_pseudoconsole, hpc, sizeof(voidptr),
		unsafe { nil }, unsafe { nil })
	mut si := StartupInfoEx{
		cb:    u32(sizeof(StartupInfoEx))
		attrs: list.data
	}
	si.dims[7] = startf_usestdhandles // no inherited std handles: output goes to the console
	mut pi := ProcessInfo{}
	cmd := if opts.command != '' { opts.command } else { os.getenv_opt('COMSPEC') or { 'cmd.exe' } }
	cwd := if opts.cwd != '' { opts.cwd.to_wide() } else { unsafe { &u16(nil) } }
	block := env_block(opts.env)
	mut env := unsafe { nil }
	if block.len > 0 {
		env = voidptr(block.data)
	}
	if !C.CreateProcessW(unsafe { nil }, cmd.to_wide(), unsafe { nil }, unsafe { nil }, false,
		extended_startupinfo_present | create_unicode_environment, env, cwd, &si, &pi) {
		code := C.GetLastError()
		api.close(hpc)
		C.CloseHandle(in_write)
		C.CloseHandle(out_read)
		return error('CreateProcessW(${cmd}) failed: ${code}')
	}
	C.CloseHandle(pi.thread)
	mut p := &ConPty{
		api:     api
		hpc:     hpc
		input:   in_write
		output:  out_read
		process: pi.process
	}
	spawn p.writer()
	reader := spawn read_loop(out_read, opts.ctx, opts.on_data)
	spawn wait_loop(mut p, reader, opts.ctx, opts.on_exit)
	return p
}

// env_block is the UTF-16 environment of the child: the parent's plus `extra`, sorted the way
// CreateProcess wants it. Empty when there is nothing to change.
fn env_block(extra map[string]string) []u16 {
	if extra.len == 0 {
		return []
	}
	mut block := []u16{}
	for line in platform.merged_env(os.environ(), extra) {
		w := line.to_wide()
		mut i := 0
		for unsafe { w[i] } != 0 {
			block << unsafe { w[i] }
			i++
		}
		block << 0
	}
	block << 0
	return block
}

fn read_loop(h voidptr, ctx prim.Holder, on_data fn (ctx prim.Holder, data []u8)) {
	mut buf := []u8{len: 65536}
	for {
		mut got := u32(0)
		if !C.ReadFile(h, buf.data, u32(buf.len), &got, unsafe { nil }) || got == 0 {
			return
		}
		if on_data != unsafe { nil } {
			on_data(ctx, buf[..int(got)]) // lent: the inbox copies it, so a read allocates nothing
		}
	}
}

// wait_loop waits for the program, then closes the pseudo console (which ends the reader
// once the last output is read) and reports the exit code after the data.
fn wait_loop(mut p ConPty, reader thread, ctx prim.Holder, on_exit fn (ctx prim.Holder, code int)) {
	C.WaitForSingleObject(p.process, -1)
	mut code := u32(0)
	C.GetExitCodeProcess(p.process, &code)
	p.shut(false)
	reader.wait()
	C.CloseHandle(p.output)
	C.CloseHandle(p.process)
	if on_exit != unsafe { nil } {
		on_exit(ctx, int(code))
	}
}

pub fn (mut p ConPty) write(data []u8) ! {
	p.mu.@lock()
	defer {
		p.mu.unlock()
	}
	if p.closed || data.len == 0 {
		return
	}
	if p.queue.len >= platform.pty_write_items || p.queued + data.len > platform.pty_write_budget {
		return error('PTY write queue is full')
	}
	p.queue << data.clone()
	p.queued += data.len
	p.wake.post()
}

// writer does the blocking writes, so a program that stops reading never stalls the UI
// thread; it owns the input handle and closes it when the session closes.
fn (mut p ConPty) writer() {
	for {
		p.wake.wait()
		p.mu.@lock()
		closed := p.closed
		mut item := []u8{}
		if p.queue.len > 0 {
			item = p.queue.first()
			p.queue.delete(0)
			p.queued -= item.len
		}
		p.mu.unlock()
		mut done := u32(0)
		if closed || (item.len > 0
			&& !C.WriteFile(p.input, item.data, u32(item.len), &done, unsafe { nil })) {
			break
		}
	}
	C.CloseHandle(p.input)
}

pub fn (mut p ConPty) resize(cols int, rows int) ! {
	p.mu.@lock()
	defer {
		p.mu.unlock()
	}
	if p.closed {
		return
	}
	hr := p.api.resize(p.hpc, coord(cols, rows))
	if hr != 0 {
		return error('ResizePseudoConsole failed: 0x${u32(hr).hex()}')
	}
}

// close ends the session: closing the pseudo console asks the programs attached to it to exit.
// The program still running child_exit_grace_ms later is terminated, from a thread of its own.
pub fn (mut p ConPty) close() {
	p.shut(true)
}

fn (mut p ConPty) shut(reap bool) {
	p.mu.@lock()
	if p.closed {
		p.mu.unlock()
		return
	}
	p.closed = true
	// The reaper gets its own handle: wait_loop closes p.process once the program ends, but only
	// after this lock is released, so the handle is still open here.
	mut own := unsafe { nil }
	if reap {
		me := C.GetCurrentProcess()
		C.DuplicateHandle(me, p.process, me, &own, 0, 0, duplicate_same_access)
	}
	p.mu.unlock()
	p.api.close(p.hpc)
	p.wake.post()
	if own != unsafe { nil } {
		spawn reap_child(own)
	}
}

// reap_child terminates a closed session's program that outlived the grace period. Only this
// process's own child, through the handle it holds for it.
fn reap_child(process voidptr) {
	if C.WaitForSingleObject(process, child_exit_grace_ms) == wait_timeout {
		eprintln('conpty: the shell outlived its closed pseudo console by ${child_exit_grace_ms} ms; terminating it')
		C.TerminateProcess(process, 1)
	}
	C.CloseHandle(process)
}

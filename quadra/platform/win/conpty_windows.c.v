// ConPTY: a pseudo console with a reader thread (specs/17). The pseudo console functions and
// the attribute-list helpers are loaded at run time, so older toolchains still build.
module win

import os
import sync
import quadra.platform

type FnCreatePC = fn (size u32, input voidptr, output voidptr, flags u32, out &voidptr) int

type FnResizePC = fn (hpc voidptr, size u32) int

type FnClosePC = fn (hpc voidptr)

type FnInitAttrs = fn (list voidptr, count u32, flags u32, size &usize) int

type FnUpdateAttr = fn (list voidptr, flags u32, attr usize, value voidptr, size usize, prev voidptr, ret voidptr) int

const proc_thread_attribute_pseudoconsole = usize(0x00020016)
const extended_startupinfo_present = u32(0x00080000)
const startf_usestdhandles = u32(0x00000100)

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
}

fn pc_api() !PcApi {
	k := load_system_dll('kernel32.dll')!
	return PcApi{
		create: unsafe { FnCreatePC(dll_proc(k, 'CreatePseudoConsole')!) }
		resize: unsafe { FnResizePC(dll_proc(k, 'ResizePseudoConsole')!) }
		close:  unsafe { FnClosePC(dll_proc(k, 'ClosePseudoConsole')!) }
		init:   unsafe { FnInitAttrs(dll_proc(k, 'InitializeProcThreadAttributeList')!) }
		update: unsafe { FnUpdateAttr(dll_proc(k, 'UpdateProcThreadAttribute')!) }
	}
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
}

fn coord(cols int, rows int) u32 {
	return u32(u16(cols)) | (u32(u16(rows)) << 16)
}

pub fn spawn_conpty(opts platform.PtyOptions) !platform.Pty {
	api := pc_api() or { return error('ConPTY needs Windows 10 1809 or later: ${err}') }
	mut in_read, mut in_write := unsafe { nil }, unsafe { nil }
	mut out_read, mut out_write := unsafe { nil }, unsafe { nil }
	if !C.CreatePipe(&in_read, &in_write, unsafe { nil }, 0)
		|| !C.CreatePipe(&out_read, &out_write, unsafe { nil }, 0) {
		return error('CreatePipe failed: ${C.GetLastError()}')
	}
	mut hpc := unsafe { nil }
	hr := api.create(coord(opts.cols, opts.rows), in_read, out_write, 0, &hpc)
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
	if !C.CreateProcessW(unsafe { nil }, cmd.to_wide(), unsafe { nil }, unsafe { nil }, false,
		extended_startupinfo_present, unsafe { nil }, cwd, &si, &pi) {
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
	reader := spawn read_loop(out_read, opts.on_data)
	spawn wait_loop(mut p, reader, opts.on_exit)
	return p
}

fn read_loop(h voidptr, on_data fn (data []u8)) {
	mut buf := []u8{len: 65536}
	for {
		mut got := u32(0)
		if !C.ReadFile(h, buf.data, u32(buf.len), &got, unsafe { nil }) || got == 0 {
			return
		}
		if on_data != unsafe { nil } {
			on_data(buf[..int(got)].clone())
		}
	}
}

// wait_loop waits for the program, then closes the pseudo console (which ends the reader
// once the last output is read) and reports the exit code after the data.
fn wait_loop(mut p ConPty, reader thread, on_exit fn (code int)) {
	C.WaitForSingleObject(p.process, -1)
	mut code := u32(0)
	C.GetExitCodeProcess(p.process, &code)
	p.close()
	reader.wait()
	C.CloseHandle(p.output)
	C.CloseHandle(p.process)
	if on_exit != unsafe { nil } {
		on_exit(int(code))
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
	mut done := u32(0)
	if !C.WriteFile(p.input, data.data, u32(data.len), &done, unsafe { nil }) {
		return error('WriteFile failed: ${C.GetLastError()}')
	}
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

// close ends the session: closing the pseudo console ends the programs attached to it.
pub fn (mut p ConPty) close() {
	p.mu.@lock()
	if p.closed {
		p.mu.unlock()
		return
	}
	p.closed = true
	p.mu.unlock()
	p.api.close(p.hpc)
	C.CloseHandle(p.input)
}

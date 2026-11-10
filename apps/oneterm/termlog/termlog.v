// Per-terminal printable-output logging (specs/25). The session pump feeds raw bytes to process;
// only the text a human would have seen reaches the file.
module termlog

import os
import sync
import time

pub const log_file_name_format = '%n_%Y-%m-%d_%H-%M-%S.log'
pub const log_content_format = '[%Y-%m-%d %H:%M:%S] %msg'

fn C.write(fd int, buf voidptr, n usize) int
fn C.close(fd int) int

pub enum LogWriteMode {
	append    // preserve existing content and write at its end
	overwrite // truncate once when logging starts
}

// TerminalLogConfig is the startup configuration resolved for one terminal.
pub struct TerminalLogConfig {
pub:
	enabled    bool // start logging before output is pumped
	directory  string
	write_mode LogWriteMode
}

// default_terminal_log_dir is `<home>/.OneTerm/logs`.
pub fn default_terminal_log_dir(home string) string {
	return os.join_path(home, '.OneTerm', 'logs')
}

pub enum LogStateKind {
	stopped // no file is open
	running // output is being written to `path`
	failed  // stopped after a setup or write failure; `message` says why
}

pub struct TerminalLogState {
pub:
	kind    LogStateKind
	path    string
	message string
}

struct ActiveLogger {
mut:
	path      string
	fd        int
	collector LineCollector
}

// TerminalLogController is shared by the pump and the UI; every call takes the mutex.
@[heap]
pub struct TerminalLogController {
mut:
	mu            &sync.Mutex = sync.new_mutex()
	identity      string
	state         TerminalLogState
	active        &ActiveLogger = unsafe { nil }
	pending_error string
}

// new_controller makes a stopped logger with an initial file name identity.
pub fn new_controller(identity string) &TerminalLogController {
	return &TerminalLogController{
		identity: if identity == '' { 'terminal' } else { identity }
	}
}

// set_identity replaces the identity before logging starts.
pub fn (mut c TerminalLogController) set_identity(identity string) {
	c.mu.@lock()
	defer {
		c.mu.unlock()
	}
	if isnil(c.active) {
		c.identity = identity
	}
}

// start opens the log file and begins capturing. While active it returns the same path.
pub fn (mut c TerminalLogController) start(config TerminalLogConfig) !string {
	c.mu.@lock()
	defer {
		c.mu.unlock()
	}
	if !isnil(c.active) {
		return c.active.path
	}
	active := open_logger(c.identity, config) or {
		message := 'Failed to start terminal logging: ${err.msg()}'
		c.fail(message)
		return error(message)
	}
	c.active = active
	c.state = TerminalLogState{
		kind: .running
		path: active.path
	}
	c.pending_error = ''
	return active.path
}

// stop flushes the final partial line and closes the file.
pub fn (mut c TerminalLogController) stop() ! {
	c.mu.@lock()
	defer {
		c.mu.unlock()
	}
	if isnil(c.active) {
		c.state = TerminalLogState{}
		c.pending_error = ''
		return
	}
	mut active := c.active
	c.active = unsafe { nil }
	active.collector.commit()
	defer {
		C.close(active.fd)
	}
	write_completed(mut active) or {
		message := 'Failed to stop terminal logging: ${err.msg()}'
		c.fail(message)
		return error(message)
	}
	c.state = TerminalLogState{}
	c.pending_error = ''
}

// state returns the current logging state.
pub fn (mut c TerminalLogController) state() TerminalLogState {
	c.mu.@lock()
	defer {
		c.mu.unlock()
	}
	return c.state
}

// take_error returns a new failure message once, for the user notification; '' when none.
pub fn (mut c TerminalLogController) take_error() string {
	c.mu.@lock()
	defer {
		c.mu.unlock()
	}
	message := c.pending_error
	c.pending_error = ''
	return message
}

// process feeds one transport read through the printable-output stripper.
pub fn (mut c TerminalLogController) process(bytes []u8) {
	c.mu.@lock()
	defer {
		c.mu.unlock()
	}
	if isnil(c.active) {
		return
	}
	mut active := c.active
	active.collector.feed(bytes)
	write_completed(mut active) or {
		C.close(active.fd)
		c.active = unsafe { nil }
		c.fail('Terminal logging stopped: ${err.msg()}')
	}
}

fn (mut c TerminalLogController) fail(message string) {
	eprintln(message)
	c.state = TerminalLogState{
		kind:    .failed
		message: message
	}
	c.pending_error = message
}

fn open_logger(identity string, config TerminalLogConfig) !&ActiveLogger {
	os.mkdir_all(config.directory)!
	t := time.now()
	path := os.join_path(config.directory,
		'${sanitize_identity(identity)}_${t.year:04}-${t.month:02}-${t.day:02}_${t.hour:02}-${t.minute:02}-${t.second:02}.log')
	return &ActiveLogger{
		path: path
		fd:   open_log_fd(path, config.write_mode)!
	}
}

// A plain descriptor: writes are unbuffered, so a failed one is seen. os.open_append leaves
// File.fd unset and os.open_file pulls in File.seek, which llvm-mingw does not compile.
fn open_log_fd(path string, mode LogWriteMode) !int {
	mut fd := -1
	$if windows {
		// _O_WRONLY | _O_CREAT | _O_BINARY, and _O_APPEND or _O_TRUNC (vlib keeps these private).
		flags := 0x0001 | 0x0100 | 0x8000 | if mode == .append { 0x0008 } else { 0x0200 }
		fd = C._wopen(path.to_wide(), flags, 0o600)
	} $else {
		tail := if mode == .append { C.O_APPEND } else { C.O_TRUNC }
		fd = C.open(&char(path.str), C.O_WRONLY | C.O_CREAT | tail, 0o600)
	}
	if fd < 0 {
		return error('cannot open ${path}: ${os.posix_get_error_msg(C.errno)}')
	}
	return fd
}

fn write_completed(mut active ActiveLogger) ! {
	for message in active.collector.take_completed() {
		t := time.now()
		line := '[${t.year:04}-${t.month:02}-${t.day:02} ${t.hour:02}:${t.minute:02}:${t.second:02}] ${message}\n'
		if C.write(active.fd, line.str, usize(line.len)) != line.len {
			return error('write failed')
		}
	}
}

// sanitize_identity keeps the file name inside the configured folder.
pub fn sanitize_identity(identity string) string {
	mut out := []u8{cap: identity.len}
	for ch in identity {
		out << if ch.is_alnum() || ch in [`-`, `_`, `.`] { ch } else { u8(`_`) }
	}
	sanitized := out.bytestr().trim('.')
	return if sanitized == '' { 'terminal' } else { sanitized }
}

// local_log_identity is `<process>_<pid>` for a local shell.
pub fn local_log_identity(program string, pid u32) string {
	name := os.file_name(program)
	return '${if name == '' {
		'shell'
	} else {
		name
	}}_${pid}'
}

// ssh_log_identity is `<user>_<host>_<port>` for SSH.
pub fn ssh_log_identity(username string, host string, port u16) string {
	return '${username}_${host}_${port}'
}

// Leveled logging to a daily file with a size cap, plus an in-memory ring the devtools
// console reads (specs/10). Safe to call from worker threads.
module app

import os
import sync
import time

pub enum Level {
	debug
	info
	warn
	error
}

pub struct LogLine {
pub:
	at    time.Time
	level Level
	scope string
	msg   string
}

@[heap]
pub struct Logger {
mut:
	lock &sync.Mutex = sync.new_mutex()
	ring []LogLine
	file os.File
	day  string
	size i64
	once map[string]bool
pub:
	dir      string
	app_id   string
	min      Level = .info
	ring_cap int   = 500
	max_size i64   = 5 * 1024 * 1024 // per daily file; a full file is renamed to .1 (one backup)
}

// new_logger writes to dir/<app_id>-YYYY-MM-DD.log; an empty dir logs to memory only.
pub fn new_logger(dir string, app_id string, min Level) &Logger {
	return &Logger{
		dir:    dir
		app_id: app_id
		min:    min
	}
}

pub fn (mut l Logger) debug(scope string, msg string) {
	l.write(.debug, scope, msg)
}

pub fn (mut l Logger) info(scope string, msg string) {
	l.write(.info, scope, msg)
}

pub fn (mut l Logger) warn(scope string, msg string) {
	l.write(.warn, scope, msg)
}

pub fn (mut l Logger) error(scope string, msg string) {
	l.write(.error, scope, msg)
}

// warn_once logs a message the first time a key is seen (missing translations).
pub fn (mut l Logger) warn_once(key string, scope string, msg string) {
	l.lock.@lock()
	seen := key in l.once
	l.once[key] = true
	l.lock.unlock()
	if !seen {
		l.write(.warn, scope, msg)
	}
}

// recent returns up to n of the newest lines, oldest first.
pub fn (mut l Logger) recent(n int) []LogLine {
	l.lock.@lock()
	defer {
		l.lock.unlock()
	}
	start := if l.ring.len > n { l.ring.len - n } else { 0 }
	return l.ring[start..].clone()
}

pub fn (mut l Logger) write(level Level, scope string, msg string) {
	if int(level) < int(l.min) {
		return
	}
	line := LogLine{time.now(), level, scope, msg}
	l.lock.@lock()
	defer {
		l.lock.unlock()
	}
	l.ring << line
	if l.ring.len > l.ring_cap {
		l.ring = l.ring[l.ring.len - l.ring_cap..].clone()
	}
	if l.dir != '' {
		l.append_file(line)
	}
}

pub fn (line LogLine) str() string {
	return '${line.at.format_ss_milli()} ${line.level.str().to_upper():-5} [${line.scope}] ${line.msg}'
}

fn (mut l Logger) append_file(line LogLine) {
	day := line.at.ymmdd()
	if day != l.day {
		l.file.close()
		l.day = day
		path := l.path()
		l.size = if os.exists(path) { os.file_size(path) } else { u64(0) } // file_size prints on a missing file
		l.file = os.open_append(path) or { return }
	}
	text := line.str() + '\n'
	l.file.write_string(text) or { return }
	l.file.flush()
	l.size += text.len
	if l.size > l.max_size {
		l.file.close()
		path := l.path()
		os.mv(path, path + '.1', overwrite: true) or {}
		l.size = 0
		l.file = os.open_append(path) or { return }
	}
}

fn (l &Logger) path() string {
	return os.join_path(l.dir, '${l.app_id}-${l.day}.log')
}

// close flushes and closes the log file.
pub fn (mut l Logger) close() {
	l.lock.@lock()
	l.file.close()
	l.day = ''
	l.lock.unlock()
}

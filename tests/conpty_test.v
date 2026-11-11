// ConPTY smoke test: a real program on a pseudo console. Skipped where ConPTY is missing.
module main

import os
import sync
import time
import quadra.platform
import quadra.prim
import quadra.platform.win

struct Sink {
mut:
	mu   &sync.Mutex = sync.new_mutex()
	out  []u8
	code int = -1
}

fn start(command string, sink &Sink, env map[string]string) !platform.Pty {
	return win.spawn_conpty(platform.PtyOptions{
		command: command
		env:     env
		cols:    80
		rows:    24
		ctx:     prim.hold(sink)
		on_data: fn (ctx prim.Holder, data []u8) {
			mut sink := prim.held[Sink](ctx)
			sink.mu.@lock()
			sink.out << data
			sink.mu.unlock()
		}
		on_exit: fn (ctx prim.Holder, code int) {
			mut sink := prim.held[Sink](ctx)
			sink.mu.@lock()
			sink.code = code
			sink.mu.unlock()
		}
	})
}

// wait_exit is true once the program has exited, within 5 s.
fn wait_exit(mut sink Sink) bool {
	for _ in 0 .. 500 {
		sink.mu.@lock()
		done := sink.code >= 0
		sink.mu.unlock()
		if done {
			return true
		}
		time.sleep(10 * time.millisecond)
	}
	return false
}

fn test_conpty_runs_a_program() {
	mut sink := &Sink{}
	mut pty := start('cmd.exe /c echo quadra-ok', sink, {}) or {
		eprintln('skipped: ${err}')
		return
	}
	pty.resize(100, 30) or { panic(err) }
	assert wait_exit(mut sink), 'the program did not exit within 5 s'
	sink.mu.@lock()
	out := sink.out.bytestr()
	code := sink.code
	sink.mu.unlock()
	assert out.contains('quadra-ok'), 'output: ${out}'
	assert code == 0
	pty.close() // after exit: a no-op
}

// Writes go through the writer thread: typed input reaches the program and runs.
fn test_conpty_write_reaches_the_program() {
	mut sink := &Sink{}
	mut pty := start('cmd.exe', sink, {}) or {
		eprintln('skipped: ${err}')
		return
	}
	pty.write('echo quadra-writer\r'.bytes()) or { panic(err) }
	pty.write('exit 7\r'.bytes()) or { panic(err) }
	assert wait_exit(mut sink), 'the program did not read its input within 5 s'
	sink.mu.@lock()
	out := sink.out.bytestr()
	code := sink.code
	sink.mu.unlock()
	assert out.contains('quadra-writer'), 'output: ${out}'
	assert code == 7
	pty.close()
}

// Closing with writes still queued returns: the writer gives up with the session.
fn test_conpty_closes_with_writes_queued() {
	mut sink := &Sink{}
	mut pty := start('cmd.exe', sink, {}) or {
		eprintln('skipped: ${err}')
		return
	}
	for _ in 0 .. 200 {
		pty.write('rem filler\r'.bytes()) or { break }
	}
	pty.close()
	assert wait_exit(mut sink), 'the session did not end within 5 s'
}

// A variable with a non-ASCII value used to cost the ones after it: with extra variables
// the child gets a block built from os.environ, and lost PATH that way.
fn test_a_non_ascii_variable_keeps_the_next_one() {
	os.setenv('QUADRA_ENV_A', 'Tiếng Việt ở đây', true)
	os.setenv('QUADRA_ENV_B', 'still-here', true)
	assert os.environ()['QUADRA_ENV_B'] == 'still-here'
	mut sink := &Sink{}
	mut pty := start('cmd.exe /c echo [%QUADRA_ENV_B%]', sink, {
		'LANG': 'en_US.UTF-8'
	}) or {
		eprintln('skipped: ${err}')
		return
	}
	assert wait_exit(mut sink), 'the program did not exit within 5 s'
	sink.mu.@lock()
	out := sink.out.bytestr()
	sink.mu.unlock()
	assert out.contains('[still-here]'), 'output: ${out}'
	pty.close()
}

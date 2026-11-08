// ConPTY smoke test: a real program on a pseudo console. Skipped where ConPTY is missing.
module main

import sync
import time
import quadra.platform
import quadra.platform.win

struct Sink {
mut:
	mu   &sync.Mutex = sync.new_mutex()
	out  []u8
	code int = -1
}

fn test_conpty_runs_a_program() {
	mut sink := &Sink{}
	mut pty := win.spawn_conpty(platform.PtyOptions{
		command: 'cmd.exe /c echo quadra-ok'
		cols:    80
		rows:    24
		on_data: fn [mut sink] (data []u8) {
			sink.mu.@lock()
			sink.out << data
			sink.mu.unlock()
		}
		on_exit: fn [mut sink] (code int) {
			sink.mu.@lock()
			sink.code = code
			sink.mu.unlock()
		}
	}) or {
		eprintln('skipped: ${err}')
		return
	}
	pty.resize(100, 30) or { panic(err) }
	mut done := false
	for _ in 0 .. 500 {
		sink.mu.@lock()
		done = sink.code >= 0
		sink.mu.unlock()
		if done {
			break
		}
		time.sleep(10 * time.millisecond)
	}
	assert done, 'the program did not exit within 5 s'
	sink.mu.@lock()
	out := sink.out.bytestr()
	code := sink.code
	sink.mu.unlock()
	assert out.contains('quadra-ok'), 'output: ${out}'
	assert code == 0
	pty.close() // after exit: a no-op
}

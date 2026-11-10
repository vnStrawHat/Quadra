// The real shell integration on a real pseudo console. Skipped where ConPTY is missing.
module main

import sync
import time
import quadra.platform
import quadra.platform.win

struct Sink {
mut:
	mu  &sync.Mutex = sync.new_mutex()
	out []u8
}

fn (mut s Sink) text() string {
	s.mu.@lock()
	defer {
		s.mu.unlock()
	}
	return s.out.bytestr()
}

fn test_cmd_reports_its_prompt_and_gets_the_environment() {
	spec := resolve_shell(ShellConfig{}, 'cmd', '')
	mut sink := &Sink{}
	mut pty := win.spawn_conpty(platform.PtyOptions{
		command: spec.command
		env:     spec.env
		cols:    120
		rows:    30
		ctx:     sink
		on_data: fn (ctx voidptr, data []u8) {
			mut sink := unsafe { &Sink(ctx) }
			sink.mu.@lock()
			sink.out << data
			sink.mu.unlock()
		}
		on_exit: fn (_ voidptr, code int) {}
	}) or {
		eprintln('skipped: ${err}')
		return
	}
	defer {
		pty.close()
	}
	wait_for(mut sink, ']133;B')
	pty.write('echo [%TERM_PROGRAM%]\r\n'.bytes()) or { panic(err) }
	wait_for(mut sink, '[OneTerm]')
	assert sink.text().contains(']7;')
}

fn wait_for(mut sink Sink, s string) {
	for _ in 0 .. 500 {
		if sink.text().contains(s) {
			return
		}
		time.sleep(10 * time.millisecond)
	}
	assert false, 'never saw ${s}: ${sink.text()}'
}

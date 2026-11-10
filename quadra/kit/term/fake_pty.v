// FakePty: a pseudo terminal for tests and demos. It records what the terminal sends,
// echoes typing like a cooked line discipline, and lets a script answer each line.
module term

import quadra.core
import quadra.platform

pub type LineFn = fn (line string) string

@[heap]
pub struct FakePty {
pub mut:
	written []u8 // everything the terminal sent
	cols    int
	rows    int
	echo    bool = true
	closed  bool
	answer  LineFn = unsafe { nil } // output for each entered line (a fake shell)
	reply   core.Bound[string, string] // answer with data, no closure (core.bind_fn): wins over answer
	prompt  string
	opts    platform.PtyOptions // what the terminal asked for
mut:
	line []u8
}

// spawner returns a spawn function that hands out this fake (tests keep the pointer). It is
// a closure: code that starts terminals from handlers uses start_with and spawn_fake.
pub fn (mut f FakePty) spawner() fn (opts platform.PtyOptions) !platform.Pty {
	mut fp := unsafe { f }
	// A test fixture: tests make one per fake, at set-up, never per event (apps use spawn_fake).
	return fn [mut fp] (opts platform.PtyOptions) !platform.Pty {
		return spawn_fake(FakeRef{fp}, opts)
	}
}

// FakeRef is the data start_with passes to spawn_fake.
pub struct FakeRef {
pub:
	fake &FakePty
}

// spawn_fake hands out the fake as the terminal's program.
pub fn spawn_fake(r FakeRef, opts platform.PtyOptions) !platform.Pty {
	mut fp := r.fake
	fp.opts = opts
	fp.cols, fp.rows = opts.cols, opts.rows
	if fp.prompt != '' {
		fp.output(fp.prompt)
	}
	return fp
}

// output sends program output to the terminal.
pub fn (mut f FakePty) output(s string) {
	if f.opts.on_data != unsafe { nil } && !f.closed {
		f.opts.on_data(f.opts.ctx, s.bytes())
	}
}

// exit ends the fake program with a code.
pub fn (mut f FakePty) exit(code int) {
	if !f.closed && f.opts.on_exit != unsafe { nil } {
		f.closed = true
		f.opts.on_exit(f.opts.ctx, code)
	}
}

pub fn (mut f FakePty) write(data []u8) ! {
	f.written << data
	if !f.echo {
		return
	}
	for b in data {
		match b {
			`\r` {
				f.output('\r\n')
				line := f.line.bytestr()
				f.line.clear()
				if f.reply.is_set() {
					f.output(f.reply.run(line))
				} else if f.answer != unsafe { nil } {
					f.output(f.answer(line))
				}
				f.output(f.prompt)
			}
			0x7f {
				if f.line.len > 0 {
					f.line.delete_last()
					f.output('\b \b')
				}
			}
			else {
				if b >= 0x20 && b != 0x1b {
					f.line << b
					f.output([b].bytestr())
				}
			}
		}
	}
}

pub fn (mut f FakePty) resize(cols int, rows int) ! {
	f.cols, f.rows = cols, rows
}

pub fn (mut f FakePty) close() {
	f.exit(0)
}

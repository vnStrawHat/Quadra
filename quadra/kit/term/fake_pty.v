// FakePty: a pseudo terminal for tests and demos. It records what the terminal sends,
// echoes typing like a cooked line discipline, and lets a script answer each line.
module term

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
	prompt  string
mut:
	on_data fn (data []u8) = unsafe { nil }
	on_exit fn (code int)  = unsafe { nil }
	line    []u8
}

// spawner returns a spawn function that hands out this fake (tests keep the pointer).
pub fn (mut f FakePty) spawner() fn (opts platform.PtyOptions) !platform.Pty {
	mut fp := unsafe { f }
	return fn [mut fp] (opts platform.PtyOptions) !platform.Pty {
		fp.on_data, fp.on_exit = opts.on_data, opts.on_exit
		fp.cols, fp.rows = opts.cols, opts.rows
		if fp.prompt != '' {
			fp.output(fp.prompt)
		}
		return fp
	}
}

// output sends program output to the terminal.
pub fn (mut f FakePty) output(s string) {
	if f.on_data != unsafe { nil } && !f.closed {
		f.on_data(s.bytes())
	}
}

// exit ends the fake program with a code.
pub fn (mut f FakePty) exit(code int) {
	if !f.closed && f.on_exit != unsafe { nil } {
		f.closed = true
		f.on_exit(code)
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
				if f.answer != unsafe { nil } {
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

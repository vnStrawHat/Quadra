// Helper for tests/crash_test.v: installs the crash handler, then dereferences nil.
module main

import os
import quadra.platform.win

fn main() {
	win.install_crash_handler(os.args[1], fn () string {
		return 'last log lines'
	})
	unsafe {
		mut p := &int(nil)
		*p = 42
	}
}

module main

import os
import time
import quadra.core
import quadra.native
import quadra.platform

fn C.GetProcessTimes(h voidptr, c &u64, e &u64, k &u64, u &u64) int
fn C.GetCurrentProcess() voidptr

// cpu_ms returns user + kernel CPU time of this process.
fn cpu_ms() f64 {
	mut c, mut e, mut k, mut u := u64(0), u64(0), u64(0), u64(0)
	C.GetProcessTimes(C.GetCurrentProcess(), &c, &e, &k, &u)
	return f64(k + u) / 10000.0
}

fn main() {
	mut app := core.new_app(native.backends(force_warp: '--warp' in os.args)!, core.AppOptions{})!
	app.open_window(platform.WindowOptions{ title: 'Quadra — Counter', width: 480, height: 320 }, Counter{})!
	if '--idle-check' in os.args {
		// Let the first frames settle, then measure 3 s of idleness and quit.
		spawn fn (mut app core.App) {
			time.sleep(1 * time.second)
			start := cpu_ms()
			time.sleep(3 * time.second)
			used := cpu_ms() - start
			println('idle CPU over 3 s: ${used:.1f} ms (${used / 30:.2f}% of one core)')
			app.post_main(fn (mut app core.App, _ voidptr) {
				app.quit()
			}, unsafe { nil })
		}(mut app)
	}
	app.run()
}

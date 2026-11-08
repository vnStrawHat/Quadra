// WARP-first start-up (gpu/d3d11/upgrade_windows.c.v) in a real window: the first frame
// draws on WARP, then the app moves to the hardware device and keeps drawing. On a machine
// without a hardware adapter there is no switch and the app stays on WARP.
module main

import quadra.core
import quadra.native
import quadra.platform
import quadra.ui

struct Label {
mut:
	n       int
	started bool
}

@[markused]
pub fn (mut l Label) render(mut cx core.ViewCx[Label]) core.Element {
	if !l.started { // new glyphs every 30 ms: uploads before, during and after the switch
		l.started = true
		cx.set_interval(30, fn (mut l Label, mut cx core.ViewCx[Label]) {
			l.n++
			cx.notify()
		})
	}
	return ui.div().size_full().p(20).child(ui.text('Xin chào ${l.n}'))
}

struct Seen {
mut:
	first_soft bool
	frames     int
	waiting    int
}

fn test_first_frames_on_warp_then_the_hardware_device() {
	mut app := core.new_app(native.backends()!, core.AppOptions{})!
	app.open_window(platform.WindowOptions{ title: 'up', width: 300, height: 120 }, Label{})!
	mut seen := &Seen{}
	app.present_hooks << fn [mut seen] (mut a core.App) {
		seen.first_soft = a.device.is_software()
		a.after(20, fn [mut seen] (mut a core.App) {
			check(mut seen, mut a)
		})
	}
	app.run()
	assert seen.first_soft
	assert seen.frames >= 2
}

// check waits (up to 2 s) for the switch to end, then for one more frame.
fn check(mut seen Seen, mut a core.App) {
	seen.waiting++
	if a.device.upgrading() && seen.waiting < 100 {
		a.after(20, fn [mut seen] (mut a core.App) {
			check(mut seen, mut a)
		})
		return
	}
	assert !a.device.upgrading()
	a.present_hooks << fn [mut seen] (mut a core.App) {
		seen.frames = a.windows[0].stats.frames
		a.quit()
	}
}

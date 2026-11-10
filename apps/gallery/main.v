// Quadra Gallery: every kit component on one page each.
module main

import os
import time
import apps.gallery.pages
import quadra.core
import quadra.kit.input
import quadra.kit
import quadra.native
import quadra.platform

fn main() {
	started := time.sys_mono_now()
	mut app := core.new_app(native.backends(force_warp: '--warp' in os.args)!, core.AppOptions{})!
	kit.install(mut app)!
	kit.follow_system(mut app, kit.light(), kit.dark())
	install_commands(mut app)!
	app.open_window(platform.WindowOptions{
		title:           'Quadra Gallery'
		custom_titlebar: true
		width:           1200
		height:          800
		min_width:       640
		min_height:      480
	}, pages.Gallery{
		hud: '--measure' !in os.args // keep startup and memory numbers comparable
	})!
	if '--measure' in os.args {
		measure(mut app, started)
	}
	app.run()
}

// measure prints startup time to the first presented frame and the working set, then
// quits (used by docs/progress.md; run the packaged exe for representative numbers).
fn measure(mut app core.App, started u64) {
	app.after_with(16, started, fn (started u64, mut app core.App) {
		if app.windows.len == 0 || app.windows[0].stats.frames == 0 {
			measure(mut app, started) // not presented yet: look again next frame
			return
		}
		w := app.windows[0]
		ms := f64(w.stats.first_present - started) / 1e6
		mb := f64(native.working_set_bytes()) / (1024 * 1024)
		println('startup to first presented frame ${ms:.1f} ms (from main), working set ${mb:.1f} MB, draw calls ${w.stats.draw_calls}, instances ${w.stats.instances}')
		app.quit()
	})
}

pub struct OpenPalette {}

// install_commands binds ctrl-k to the command palette, anywhere in the app.
fn install_commands(mut app core.App) ! {
	app.bind('ctrl-k', OpenPalette{})!
	app.on_action(core.handler(fn (_ &OpenPalette, mut cx core.Cx) {
		input.open_command_palette(mut cx, pages.commands())
	}))
}

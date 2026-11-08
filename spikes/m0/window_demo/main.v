// M0 manual IME check: a bare Win32 window that logs keyboard, text and IME events.
// Type Vietnamese with the Windows Telex/VNI keyboard and with Unikey/EVKey; the
// console shows `ime start/preedit/commit/end` and `text` lines.
module main

import quadra.prim
import quadra.platform
import quadra.platform.win

struct Logger {
mut:
	p &win.WinPlatform = unsafe { nil }
	w ?platform.PlatformWindow
}

fn (mut l Logger) on_event(_ platform.WindowId, e platform.Event) {
	match e {
		platform.TextEvent {
			println('text "${e.text}"')
		}
		platform.ImeEvent {
			println('ime ${e.kind} "${e.text}" cursor ${e.cursor}')
			if mut w := l.w {
				w.set_ime_rect(prim.rect(40, 40, 2, 20))
			}
		}
		platform.KeyEvent {
			if e.down {
				println('key ${e.key} mods ${e.mods}')
			}
		}
		platform.CloseRequested {
			if mut w := l.w {
				w.close()
			}
		}
		platform.WindowClosed {
			l.p.quit()
		}
		platform.ScaleEvent {
			println('scale ${e.scale}')
		}
		else {}
	}
}

fn (mut l Logger) on_frame(_ platform.WindowId) {}

fn (mut l Logger) on_timer(_ platform.TimerId) {}

fn main() {
	mut p := win.new_platform() or { panic(err) }
	mut l := &Logger{
		p: p
	}
	p.set_sink(mut l)
	l.w = p.open_window(platform.WindowOptions{
		title: 'Quadra IME check — gõ tiếng Việt ở đây'
	}) or { panic(err) }
	println('type in the window; close it to finish (session ${win.session_id()})')
	p.run()
}

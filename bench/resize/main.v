// Live-resize probe: opens a real window over a solid green backdrop window, drives a
// size-move loop with back-to-back SetWindowPos steps (as the OS modal loop does, pumping
// the window's messages between steps) and measures what the screen shows while it runs.
// A 3 px magenta strip marks the right edge of the content; a sampler thread reads the
// composed screen with BitBlt as fast as it can and measures, per sample, the gap between
// the strip and the window's visible edge (where the green starts).
//   bench/resize [out_dir] [light|dark] [paragraphs]   (paragraphs 0: the gallery)
module main

import os
import time
import apps.gallery.pages
import quadra.core
import quadra.kit
import quadra.native
import quadra.platform
import quadra.prim
import quadra.ui

const marker = prim.rgb(0xFF00FF)
const backdrop = prim.rgb(0x00C000)
const steps = 40
const step_px = 16

struct Host {
	paras int
mut:
	gallery core.View[pages.Gallery]
}

const words = 'resize keeps the content glued to the window edge while the frame follows the pointer and every line of this paragraph wraps again at the new width'.split(' ')

@[markused]
pub fn (mut h Host) render(mut cx core.ViewCx[Host]) core.Element {
	mut body := ui.div().grow().h_full().flex_col().overflow_hidden()
	if h.paras == 0 {
		if h.gallery.id == 0 {
			h.gallery = cx.new_view(pages.Gallery{})
		}
		body = body.child(cx.child_view(h.gallery))
	} else {
		body = body.p(12).gap(6)
		for i in 0 .. h.paras {
			mut s := '${i}.'
			for k in 0 .. 3 * words.len {
				s += ' ' + words[(i + k) % words.len]
			}
			body = body.child(ui.div().shrink_0().child(ui.text(s)))
		}
	}
	return ui.div().size_full().flex_row().bg(prim.rgb(0xF4F4F4)).child(body).child(ui.div().w(3).h_full().bg(marker))
}

struct Backdrop {}

@[markused]
pub fn (mut b Backdrop) render(mut cx core.ViewCx[Backdrop]) core.Element {
	return ui.div().size_full().bg(backdrop)
}

struct Run {
	out string
}

fn main() {
	out := if os.args.len > 1 { os.args[1] } else { '.tmp/resize-shots' }
	dark := os.args.len > 2 && os.args[2] == 'dark'
	paras := if os.args.len > 3 { os.args[3].int() } else { 600 }
	os.mkdir_all(out)!
	mut app := core.new_app(native.backends()!, core.AppOptions{})!
	kit.install(mut app)!
	kit.set_theme(mut app, if dark { kit.dark() } else { kit.light() })
	app.open_window(platform.WindowOptions{
		title:    'backdrop'
		width:    1700
		height:   900
		centered: true
	}, Backdrop{})!
	app.open_window(platform.WindowOptions{
		title:           'Quadra resize probe'
		custom_titlebar: true
		width:           800
		height:          600
		centered:        true
	}, Host{
		paras: paras
	})!
	app.after_with(2000, Run{out}, drive)
	app.run()
}

struct Step {
mut:
	t_ms       f64 // since the first step
	call_us    i64 // SetWindowPos: WM_SIZE through Present, all synchronous
	pump_us    i64 // the window's queued messages (WM_PAINT) after it
	frames     int // frames drawn during the step
	sized      bool
	phases     [4]i64
	present_us i64
}

fn drive(r Run, mut app core.App) {
	mut w := app.windows[1]
	hwnd := w.pw.native_handle()
	// The backdrop covers the screen's width; the probe starts near its left edge.
	C.SetWindowPos(app.windows[0].pw.native_handle(), unsafe { nil }, -8, 0,

		C.GetSystemMetrics(0) + 16, C.GetSystemMetrics(1) - 40, 0x0004 | 0x0010)
	C.SetWindowPos(hwnd, unsafe { nil }, 40, 60, 0, 0, 0x0001 | 0x0004 | 0x0010) // NOSIZE
	C.SetForegroundWindow(hwnd)
	mut wr := C.RECT{}
	C.GetWindowRect(hwnd, &wr)
	base_w, h := wr.right - wr.left, wr.bottom - wr.top
	mut s := new_sampler(hwnd, steps * step_px + 96) or {
		eprintln(err)
		exit(1)
	}
	s.band = os.args.len > 4 && os.args[4] == 'band'
	th := spawn sample_loop(mut s)
	time.sleep(300 * time.millisecond)
	s.settled = s.samples.len
	C.SendMessageW(hwnd, 0x0231, 0, 0) // WM_ENTERSIZEMOVE
	s.started = s.samples.len
	mut rows := []Step{}
	clock := time.new_stopwatch()
	for i in 0 .. steps {
		f0 := w.stats.frames
		mut st := Step{
			t_ms: f64(clock.elapsed().microseconds()) / 1000
		}
		sw := time.new_stopwatch()
		C.SetWindowPos(hwnd, unsafe { nil }, 0, 0, base_w + (i + 1) * step_px, h,
			0x0002 | 0x0004 | 0x0010) // NOMOVE NOZORDER NOACTIVATE
		st.call_us = sw.elapsed().microseconds()
		st.phases = w.stats.phase_us
		st.present_us = w.stats.present_us
		mut cr := C.RECT{}
		C.GetClientRect(hwnd, &cr)
		st.sized = w.surface.width() == cr.right
		pump(hwnd)
		st.pump_us = sw.elapsed().microseconds() - st.call_us
		st.frames = w.stats.frames - f0
		rows << st
	}
	s.ended = s.samples.len
	C.SendMessageW(hwnd, 0x0232, 0, 0) // WM_EXITSIZEMOVE
	time.sleep(300 * time.millisecond)
	s.stop = true
	th.wait()
	report(rows, s, f64(clock.elapsed().microseconds()) / 1000)
	s.save_worst(r.out)
	app.quit()
}

fn report(rows []Step, s &Sampler, total_ms f64) {
	println('step   t_ms call_ms pump_ms frames sized render layout prepaint paint present')
	mut sum := i64(0)
	for i, st in rows {
		println('${i:4} ${st.t_ms:6.1} ${ms(st.call_us)} ${ms(st.pump_us)} ${st.frames:6} ${st.sized:5} ${ms(st.phases[0])} ${ms(st.phases[1])} ${ms(st.phases[2])} ${ms(st.phases[3])} ${ms(st.present_us)}')
		sum += st.call_us + st.pump_us
	}
	println('steps: ${rows.len} in ${total_ms:.1} ms, mean step ${f64(sum) / rows.len / 1000:.2} ms')
	s.summary()
}

fn ms(us i64) string {
	return '${f64(us) / 1000:7.2}'
}

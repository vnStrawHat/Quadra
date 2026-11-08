// Hud: a small performance overlay at the window's top right: frames per second, the last
// frame's CPU time, process CPU (share of the whole machine, as Task Manager shows it) and
// memory (private working set, Task Manager's "Memory" column, and working set).
// It samples every 500 ms, so an idle window still presents two frames a second.
module pages

import runtime
import time
import quadra.core
import quadra.prim
import quadra.kit
import quadra.native
import quadra.ui

const hud_ms = 500

pub struct Hud {
mut:
	started  bool
	win      u32
	at       u64 // time.sys_mono_now() of the last sample
	frames   int
	cpu_time u64
	fps      f64
	frame_ms f64
	cpu      f64 // percent of all logical processors
	ws       u64
	private  u64
}

@[markused]
pub fn (mut h Hud) render(mut cx core.ViewCx[Hud]) core.Element {
	if !h.started {
		h.started = true
		h.win = if cx.win != unsafe { nil } { cx.win.id } else { 0 }
		h.sample(cx.app)
		cx.set_interval(hud_ms, fn (mut h Hud, mut cx core.ViewCx[Hud]) {
			h.sample(cx.app)
			cx.notify()
		})
	}
	t := kit.theme(cx)
	return ui.div().absolute().top(t.titlebar_h + 8).right(12).flex_col().gap(2).px(10).py(6).rounded(t.radius_sm).bg(prim.rgba(0x10141ad1)).text_color(prim.rgb(0x7ee787)).text_size(t.font_size(.xs)).font(t.mono_family).test_id('hud').child(ui.text('FPS ${h.fps:5.1f}  frame ${h.frame_ms:5.2f} ms')).child(ui.text('CPU ${h.cpu:5.1f} %')).child(ui.text('Private WS ${mib(h.private)}')).child(ui.text('Working set ${mib(h.ws)}'))
}

fn (mut h Hud) sample(app &core.App) {
	now := time.sys_mono_now()
	cpu_time, ws, private := native.process_stats()
	frames := if w := app.window(h.win) { w.stats.frames } else { 0 }
	if h.at > 0 && now > h.at {
		secs := f64(now - h.at) / 1e9
		h.fps = f64(frames - h.frames) / secs
		h.cpu = f64(cpu_time - h.cpu_time) / 1e7 / secs / f64(runtime.nr_cpus()) * 100
	}
	if w := app.window(h.win) {
		h.frame_ms = f64(w.stats.last_frame_us) / 1000
	}
	h.at, h.frames, h.cpu_time, h.ws, h.private = now, frames, cpu_time, ws, private
}

fn mib(b u64) string {
	return if b == 0 { 'n/a' } else { '${f64(b) / (1024 * 1024):.1f} MB' }
}

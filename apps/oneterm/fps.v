// The FPS HUD: frames per second, the GPU's name, the graphics API and this process's GPU
// usage (specs/22).
module main

import quadra.core
import quadra.kit
import quadra.native.sys
import quadra.ui

// HudGpu is the HUD's GPU-usage sampler. The PDH query is open only while the HUD is on.
struct HudGpu {
mut:
	probe   &sys.GpuUsage = unsafe { nil }
	tried   bool // the open ran (or runs) for this showing of the HUD
	busy    bool // an open or a sample is on a worker thread
	ticks   int
	percent f32 = -1 // -1: no sample yet
}

struct GpuOpen {
	probe &sys.GpuUsage = unsafe { nil }
}

fn (mut o OneTerm) toggle_fps(mut cx core.ViewCx[OneTerm]) {
	o.ui.show_fps = !o.ui.show_fps
	o.persist_ui() or {}
	o.tick_gpu(mut cx)
}

// tick_gpu opens the query when the HUD shows, samples it every other tick and closes it
// when the HUD goes. Both PDH calls run off the UI thread, one at a time.
fn (mut o OneTerm) tick_gpu(mut cx core.ViewCx[OneTerm]) {
	if o.hud.busy {
		return
	}
	if !o.ui.show_fps {
		if o.hud.probe != unsafe { nil } {
			o.hud.probe.close()
		}
		o.hud = HudGpu{}
		return
	}
	if !o.hud.tried {
		o.hud.tried, o.hud.busy = true, true
		core.spawn_task[OneTerm, GpuOpen](mut cx, fn (_ &core.TaskCx) !GpuOpen {
			return GpuOpen{sys.open_gpu_usage() or { return error('no GPU counter') }}
		}, fn (mut o OneTerm, r &core.TaskResult[GpuOpen], mut cx core.ViewCx[OneTerm]) {
			o.hud.busy = false
			if r.ok {
				o.hud.probe = r.value.probe
			}
			cx.notify()
		})
		return
	}
	if o.hud.probe == unsafe { nil } {
		return
	}
	o.hud.ticks++
	if o.hud.ticks % 2 != 0 {
		return
	}
	o.hud.busy = true
	mut probe := o.hud.probe
	core.spawn_task[OneTerm, f32](mut cx, fn [mut probe] (_ &core.TaskCx) !f32 {
		return probe.sample() or { return error('no sample') }
	}, fn (mut o OneTerm, r &core.TaskResult[f32], mut cx core.ViewCx[OneTerm]) {
		o.hud.busy = false
		if r.ok {
			o.hud.percent = r.value
			cx.notify()
		}
	})
}

// usage_text is `n/a` with no counter, `-` until the first sample lands, then `1.8%`.
fn (h &HudGpu) usage_text() string {
	if h.tried && !h.busy && h.probe == unsafe { nil } {
		return 'n/a'
	}
	if h.percent < 0 {
		return '-'
	}
	return '${h.percent:.1f}%'
}

// device_text is the adapter's name, marked on a software device.
fn device_text(name string, software bool) string {
	if name == '' {
		return 'n/a'
	}
	if software {
		return '${name} (software)'
	}
	return name
}

// hud_rows are the four rows, in display order; a negative fps (none counted yet) shows `-`.
fn hud_rows(fps int, device string, usage string) [][]string {
	frames := if fps < 0 { '-' } else { fps.str() }
	return [['FPS', frames], ['GPU', device], ['API', 'Direct3D 11'],
		['GPU usage', usage]]
}

// fps_hud sits 12 px from the right, 12 px below the tab strip; clicks stop on it.
fn (mut o OneTerm) fps_hud(t &kit.Theme, mut cx core.ViewCx[OneTerm]) core.Element {
	device := device_text(cx.app.adapter_name(), cx.app.device.is_software())
	rows := hud_rows(o.fps, device, o.hud.usage_text())
	mut col := ui.div().test_id('fps-hud').absolute().top(t.titlebar_h + t.tabs_h + 12).right(12).min_w(220).flex_col().px(8).py(4).rounded(t.radius_md).bg(t.p.overlay_bg).border(1).border_color(t.p.line).text_size(t.font_size(.sm)).on_mouse_down(core.handler(fn (_ &core.MouseDownEvent, mut cx core.Cx) {
		cx.stop_propagation()
	}))
	for r in rows {
		col.child(ui.div().flex_row().items_center().gap(8).child(ui.text(r[0]).family(t.mono_family).color(t.p.text_2)).child(ui.div().grow()).child(ui.text(r[1]).family(t.mono_family).color(t.p.text)))
	}
	return col
}

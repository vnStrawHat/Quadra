// Built-in devtools overlay: profiler (ctrl-shift-p), paint flashing (ctrl-shift-f)
// and the inspector (ctrl-shift-i), drawn on top of the window.
module core

import quadra.prim
import quadra.gpu
import quadra.platform
import quadra.text

pub struct DebugFlags {
pub mut:
	profiler  bool
	flashing  bool
	inspector bool
}

fn (mut w Window) debug_key(e platform.KeyEvent) bool {
	if !(e.mods.has(.ctrl) && e.mods.has(.shift)) {
		return false
	}
	match e.key {
		.p { w.debug.profiler = !w.debug.profiler }
		.f { w.debug.flashing = !w.debug.flashing }
		.i { w.debug.inspector = !w.debug.inspector }
		else { return false }
	}

	w.invalidate_paint()
	return true
}

fn (mut w Window) paint_debug(mut cx PaintCx) {
	if w.debug.flashing {
		for r in w.fresh_views {
			cx.quad(gpu.Quad{
				bounds:        r
				background:    prim.rgba(0xF59E0B22)
				border_widths: prim.edges(1)
				border_color:  prim.rgba(0xF59E0BCC)
			})
		}
	}
	if w.debug.inspector && w.mouse_in {
		path := w.hit_path(w.mouse)
		if path.len > 0 {
			hb := w.hitboxes[path[0]]
			cx.quad(gpu.Quad{
				bounds:        hb.bounds
				background:    prim.rgba(0x3B82F633)
				border_widths: prim.edges(1)
				border_color:  prim.rgba(0x2563EBFF)
			})
			b := hb.bounds
			w.debug_label(mut cx, '${b.width:.0f} x ${b.height:.0f} @ ${b.x:.0f},${b.y:.0f}', b.x,

				b.bottom() + 4)
		}
	}
	if w.debug.profiler {
		st := w.stats
		lines := [
			'frame ${f32(st.last_frame_us) / 1000:.2f} ms  build ${f32(st.last_build_us) / 1000:.2f} ms',
			'builds ${st.builds}  prepaints ${st.prepaints}  paints ${st.paints}',
			'views cached ${st.cache_hits}  nodes ${st.layout_nodes}',
			'draw calls ${st.draw_calls}  instances ${st.instances}',
			'gpu ${w.app.adapter_name()}',
		]
		for i, l in lines {
			w.debug_label(mut cx, l, 8, 8 + f32(i) * 18)
		}
	}
}

fn (mut w Window) debug_label(mut cx PaintCx, s string, x f32, y f32) {
	st := text.TextStyle{
		family: cx.text.default_family()
		size:   12
		color:  prim.white
	}
	l := cx.text.layout_line(s, st)
	cx.quad(gpu.Quad{
		bounds:     prim.rect(x, y, l.width + 12, 18)
		background: prim.rgba(0x111827E6)
		radii:      prim.corners(4)
	})
	cx.glyphs(l, x + 6, y + 13, prim.white)
}

// after_frame keeps devtools live: the profiler redraws to show fresh numbers.
fn (mut app App) after_frame(mut w Window) {
	app.frame_number++
	if !app.upgrade_watch && app.device.upgrading() {
		app.watch_upgrade()
	}
	if app.upgrade_watch {
		w.pw.set_waitable(w.surface.waitable()) // WARP-first: the swap chain comes late
	}
	if app.present_hooks.len > 0 {
		hooks := app.present_hooks
		app.present_hooks = []
		for f in hooks {
			f(mut app)
		}
	}
	if w.debug.profiler && w.stats.frames % 2 == 1 {
		w.invalidate_paint()
	}
	app.tick_animations(mut w)
	app.arm_trim()
}

// adapter_name is the GPU the app draws on (specs/03), '' when the backend cannot tell.
pub fn (app &App) adapter_name() string {
	return app.device.adapter_name()
}

// debug_counts summarizes the sizes of the app's registries and caches (leak hunting).
pub fn (app &App) debug_counts() string {
	pinned := app.store.slots.filter(it.alive && it.pinned).len
	mut s := 'pinned ${pinned} dead_lives ${app.dead_lives.len} ${app.text.cache_counts()} slots ${app.store.slots.len} obs ${app.observers.len} subs ${app.subs.len} timers ${app.timers.len} tasks ${app.tasks.len} images ${app.images.entries.len} global_actions ${app.global_actions.len}'
	for w in app.windows {
		s += ' | win state ${w.state.len} handlers ${w.handlers.len} hitboxes ${w.hitboxes.len} patches ${w.patches.len} deferred ${w.deferred.len} renders ${w.stats.renders.len} keyh ${w.key_handlers.len}'
	}
	return s
}

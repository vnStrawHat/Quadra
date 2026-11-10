// App: owns the backends, the model store and the windows; it is the platform's EventSink.
module core

import os
import quadra.prim
import quadra.gpu
import quadra.platform
import quadra.text

// Backends are the OS boundary implementations an app runs on (quadra.native, quadra.testing).
pub struct Backends {
pub:
	platform platform.Platform
	device   gpu.Device
	text     text.Backend
}

fn C.GC_set_free_space_divisor(n usize)

pub struct AppOptions {
pub:
	quit_on_last_window bool = true
	text_style          text.TextStyle
	// gc_divisor sets how far the heap may grow between collections (Boehm's free-space
	// divisor): larger collects more often in a smaller heap. 3 cut peak memory by a
	// third for a few percent CPU (docs/benchmarks.md); 0 keeps the collector's setting,
	// and the GC_FREE_SPACE_DIVISOR environment variable wins over both.
	gc_divisor int = 3
}

@[heap]
pub struct App {
pub mut:
	plat                platform.Platform
	device              gpu.Device
	text                &text.TextSystem = unsafe { nil }
	windows             []&Window
	quit_on_last_window bool
	text_style          text.TextStyle
	keymap              Keymap
	images              ImageCache
	frame_number        u64
	appearance_hooks    []fn (mut app App)                    // run when the system appearance changes
	present_hooks       []fn (mut app App)                    // run once after the next presented frame
	wrap_root           ?fn (mut cx Cx, root AnyView) AnyView // installed by kits (overlay host)
mut:
	store             Store
	globals           map[string]u32
	depth             int
	flushing          bool
	rendering         []u32
	changed_set       map[u32]bool
	changed_order     []u32
	pending           []PendingEvent
	observers         []Observer
	subs              []Subscription
	next_sub          u64
	timers            map[u64]TimerEntry
	next_focus        u64
	global_actions    []KeyHandler
	anims             map[string]Anim
	animating         map[u32]bool
	anim_timer        u64
	tasks             map[u64]TaskEntry
	next_task         u64
	debounced         map[string]platform.TimerId
	instance_handlers map[u32]AnyListener
	tray_handlers     map[u32]AnyListener // shell.v
	upgrade_watch     bool                // lifecycle.v
	rjob              RenderJob           // render tracking (tracked.v)
	dead_lives        []DeadLife          // closures of released views, freed after their window rebuilds
	trim              Trim                // memory given back after a burst (trim.v)
}

pub fn new_app(b Backends, opts AppOptions) !&App {
	$if !gcboehm ? {
		// Views, elements and frames are GC-allocated: without a collector memory only grows.
		$compile_error('quadra needs the garbage collector: build with `-gc boehm_full_opt` (tools/v.sh does)')
	}
	if opts.gc_divisor > 0 && os.getenv('GC_FREE_SPACE_DIVISOR') == '' {
		C.GC_set_free_space_divisor(usize(opts.gc_divisor))
	}
	mut app := &App{
		plat:                b.platform
		device:              b.device
		text:                text.new_system(b.text)
		quit_on_last_window: opts.quit_on_last_window
		text_style:          opts.text_style
	}
	app.plat.set_sink(mut app)
	return app
}

fn (app &App) base_text_style() text.TextStyle {
	return app.text_style
}

// open_window creates a window whose root view holds `root`.
pub fn (mut app App) open_window[T](opts platform.WindowOptions, root T) !View[T] {
	pw := app.plat.open_window(opts)!
	// The GPU surface is attached at the first frame, after the first layout: a device
	// still starting up (see native.backends) overlaps with building that frame.
	mut w := &Window{
		id:      pw.id
		app:     app
		pw:      pw
		surface: NoSurface{}
	}
	app.windows << w
	mut cx := Cx{
		app: app
		win: w
	}
	v := cx.new_view(root)
	w.root = v.any()
	if wrap := app.wrap_root {
		w.root = wrap(mut cx, v.any())
		// The wrapper owns the app's root view: closing the window releases both.
		app.store.slots[v.id].owner = w.root.id
		app.store.slots[w.root.id].owned << v.id
	}
	w.invalidate()
	return v
}

pub fn (mut app App) run() {
	app.plat.run()
}

pub fn (mut app App) quit() {
	app.plat.quit()
}

pub fn (app &App) window(id platform.WindowId) ?&Window {
	for w in app.windows {
		if w.id == id {
			return w
		}
	}
	return none
}

fn (app &App) window_of_view(id u32) ?&Window {
	wid := app.store.slots[id].window
	if wid == 0 {
		return none
	}
	return app.window(wid)
}

// render_any returns a view's element tree: the cached one when the view is clean,
// otherwise a fresh render. Every view is a cache boundary.
pub fn (mut app App) render_any(id u32, win &Window) Element {
	slot := app.store.slots[id]
	if !slot.alive || slot.render == unsafe { nil } {
		panic('quadra: render of a released or non-view model ${id}')
	}
	mut w := unsafe { win }
	if !slot.dirty {
		if el := slot.cached {
			w.stats.cache_hits++
			return el
		}
	}
	w.stats.renders[id] = w.stats.renders[id] + 1
	app.store.slots[id].dirty = false
	el := app.render_tracked(id, win, slot.render)
	app.store.slots[id].cached = el
	return el
}

fn (mut app App) window_closed(w &Window) {
	app.release_id(w.root.id)
	mut win := unsafe { w }
	win.release_tracking()
	app.windows = app.windows.filter(it.id != w.id)
	if app.windows.len == 0 && app.quit_on_last_window {
		app.plat.quit()
	}
}

// --- platform.EventSink ---

pub fn (mut app App) on_event(id platform.WindowId, e platform.Event) {
	if mut w := app.window(id) {
		w.handle(e)
	}
}

pub fn (mut app App) on_frame(id platform.WindowId) {
	if mut w := app.window(id) {
		w.draw_frame()
	}
}

// hit_test answers the platform for custom titlebars: the region of the deepest hitbox at
// the point. A button inside the caption (search, menus) is not a region, so it stays
// client and gets its clicks.
pub fn (mut app App) hit_test(id platform.WindowId, x f32, y f32) platform.HitRegion {
	w := app.window(id) or { return .client }
	path := w.hit_path(prim.pt(x, y))
	if path.len == 0 {
		return .client
	}
	return w.hitboxes[path[0]].region
}

pub fn (mut app App) on_timer(id platform.TimerId) {
	if id == app.anim_timer {
		app.advance_animations()
		return
	}
	app.on_timer_fired(id)
}
